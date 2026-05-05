use std::time::Instant;

use utils::log;

use crate::{
    constants::cs2,
    cs2::{CS2, offsets::Offsets, schema::Schema},
    os::process::Process,
};

impl CS2 {
    pub fn find_offsets(process: &Process) -> Option<Offsets> {
        let start = Instant::now();
        let mut offsets = Offsets::default();

        offsets.library.client = process.module_base_address(cs2::CLIENT_LIB)?;
        offsets.library.engine = process.module_base_address(cs2::ENGINE_LIB)?;
        offsets.library.tier0 = process.module_base_address(cs2::TIER0_LIB)?;
        offsets.library.input = process.module_base_address(cs2::INPUT_LIB)?;
        offsets.library.sdl = process.module_base_address(cs2::SDL_LIB)?;
        offsets.library.schema = process.module_base_address(cs2::SCHEMA_LIB)?;

        let Some(resource_offset) = process
            .get_interface_offset(offsets.library.engine, "GameResourceServiceClientV0")
        else {
            log::warn!("could not get offset for GameResourceServiceClient");
            return None;
        };
        offsets.interface.resource = resource_offset;

        // Entity system lives at GameResourceServiceClientV0 + 0x50.
        // The entity list (bucket pointer array) is at entity_system + 0x10.
        let entity_system: u64 = process.read(offsets.interface.resource + 0x50);
        let is_valid_ptr = |p: u64| p > 0x10000 && p >> 48 == 0;
        // Try offsets 0x10 first (canonical), then neighbours as fallback.
        let entity_list_off = [0x10u64, 0x08, 0x18, 0x20]
            .iter()
            .find(|&&off| is_valid_ptr(process.read::<u64>(entity_system + off)))
            .copied()
            .unwrap_or(0x10);
        offsets.interface.entity = entity_system + entity_list_off;

        let Some(cvar_address) = process
            .get_interface_offset(offsets.library.tier0, "VEngineCvar0")
        else {
            log::warn!("could not get convar interface offset");
            return None;
        };
        offsets.interface.cvar = cvar_address;
        let Some(input_address) = process
            .get_interface_offset(offsets.library.input, "InputSystemVersion0")
        else {
            log::warn!("could not get input interface offset");
            return None;
        };
        offsets.interface.input = input_address;

        // Each tuple: (pattern, rel32_byte_offset, instruction_size)
        let local_player_candidates: &[(&str, u64, u64)] = &[
            ("48 83 3D ? ? ? ? 00 0F 95 C0 C3", 3, 8), // original deadlocked: CMP [RIP+x],0; SETNZ AL; RET
            ("48 8B 05 ? ? ? ? 41 89 BE",       3, 7), // a2x Apr-2026: MOV RAX,[localCtrl]; MOV [R14+x],EDI
            ("48 83 3D ? ? ? ? 00 74 ? 48 8B",  3, 8), // CMP [RIP+x],0; JZ ?; MOV
            ("48 8B 05 ? ? ? ? 48 85 C0 0F 84", 3, 7), // MOV + long JZ variant
            ("48 8B 05 ? ? ? ? 48 85 C0 74 ? 8B 81", 3, 7), // MOV + null-check + field access
        ];
        let Some((lp_addr, lp_rel, lp_sz)) = local_player_candidates.iter().find_map(
            |&(pat, rel, sz)| process.scan(pat, offsets.library.client).map(|a| (a, rel, sz)),
        ) else {
            log::warn!("could not find local player offset");
            return None;
        };
        offsets.direct.local_player = process.get_relative_address(lp_addr, lp_rel, lp_sz);
        offsets.direct.button_state = process.read::<u32>(
            process
                .get_interface_function(offsets.interface.input, 19)
                + 0x14,
        ) as u64;

        let view_matrix_candidates: &[&str] = &[
            "C6 83 ? ? 00 00 01 4C 8D 05",  // original deadlocked: MOV byte;LEA R8,[RIP+x]
            "48 8D 0D ? ? ? ? 48 C1 E0 06",   // a2x Apr-2026: LEA RCX,[RIP+x]; SHL RAX,6
            "48 8D 05 ? ? ? ? 48 89 45 ? 48 8D 45", // LEA RAX,[RIP+x]
        ];
        let Some(vm_addr) = view_matrix_candidates.iter().find_map(
            |&pat| process.scan(pat, offsets.library.client),
        ) else {
            log::warn!("could not find view matrix offset");
            return None;
        };
        offsets.direct.view_matrix = process.get_relative_address(vm_addr + 0x0A, 0x0, 0x04);
        log::warn!("[sdl_diag] view_matrix=0x{:X} (scanned_addr=0x{:X})", offsets.direct.view_matrix, vm_addr);

        let Some(sdl_fn) = process
            .get_module_export(offsets.library.sdl, "SDL_GetKeyboardFocus")
        else {
            log::warn!("could not find sdl window offset");
            return None;
        };
        let step1 = process.get_relative_address(sdl_fn, 0x02, 0x06);
        let step2: u64 = process.read(step1);
        offsets.direct.sdl_window = process.get_relative_address(step2, 0x03, 0x07);
        log::warn!("[sdl_diag] sdl_fn=0x{:X} step1=0x{:X} step2=0x{:X} sdl_window_off=0x{:X}",
            sdl_fn, step1, step2, offsets.direct.sdl_window);

        // xref "lobby_mapveto"
        let gv_candidates: &[(&str, u64, u64)] = &[
            ("48 8D 05 ? ? ? ? 48 8B 00 8B 48 ? E9",       3, 7), // original deadlocked: LEA RAX,[RIP+x]
            ("48 89 15 ? ? ? ? 48 89 42",                  3, 7), // a2x Apr-2026: MOV [RIP+x],RDX
        ];
        let Some((gv_addr, gv_rel, gv_sz)) = gv_candidates.iter().find_map(
            |&(pat, rel, sz)| process.scan(pat, offsets.library.client).map(|a| (a, rel, sz)),
        ) else {
            log::warn!("could not find global vars offset");
            return None;
        };
        offsets.direct.global_vars = process.get_relative_address(gv_addr, gv_rel, gv_sz);

        let Some(ffa_address) = process
            .get_convar(offsets.interface.cvar, "mp_teammates_are_enemies")
        else {
            log::warn!("could not get mp_teammates_are_enemies convar offset");
            return None;
        };
        offsets.convar.ffa = ffa_address;
        let Some(sensitivity_address) = process
            .get_convar(offsets.interface.cvar, "sensitivity")
        else {
            log::warn!("could not get sensitivity convar offset");
            return None;
        };
        offsets.convar.sensitivity = sensitivity_address;

        let schema = Schema::new(process, offsets.library.schema)?;
        let client = schema.get_library(cs2::CLIENT_LIB)?;

        // m_steamID may appear on different controller classes depending on CS2 version.
        // Try CBasePlayerController first (most common), then the CS-specific controller.
        offsets.controller.steam_id = client
            .get_opt("CBasePlayerController", "m_steamID")
            .or_else(|| client.get_opt("CCSPlayerController", "m_steamID"))
            .unwrap_or(0);
        offsets.controller.name = client.get("CBasePlayerController", "m_iszPlayerName")?;
        offsets.controller.pawn = client.get("CBasePlayerController", "m_hPawn")?;
        offsets.controller.owner_entity = client.get("C_BaseEntity", "m_hOwnerEntity")?;
        offsets.controller.color = client.get("CCSPlayerController", "m_iCompTeammateColor")?;
        offsets.controller.action_tracking_services =
            client.get("CCSPlayerController", "m_pActionTrackingServices")?;
        // m_nPing may appear on different controller classes depending on CS2 version.
        // Try CBasePlayerController first (most common), then the CS-specific controller,
        // and fall back to the legacy m_iPing name if m_nPing is absent.
        offsets.controller.ping = client
            .get_opt("CBasePlayerController", "m_nPing")
            .or_else(|| client.get_opt("CCSPlayerController", "m_nPing"))
            .or_else(|| client.get_opt("CBasePlayerController", "m_iPing"))
            .or_else(|| client.get_opt("CCSPlayerController", "m_iPing"))
            .unwrap_or(0);

        offsets.pawn.health = client.get("C_BaseEntity", "m_iHealth")?;
        offsets.pawn.armor = client.get("C_CSPlayerPawn", "m_ArmorValue")?;
        offsets.pawn.team = client.get("C_BaseEntity", "m_iTeamNum")?;
        offsets.pawn.life_state = client.get("C_BaseEntity", "m_lifeState")?;
        offsets.pawn.fov_multiplier = client.get("C_BasePlayerPawn", "m_flFOVSensitivityAdjust")?;
        offsets.pawn.game_scene_node = client.get("C_BaseEntity", "m_pGameSceneNode")?;
        offsets.pawn.eye_offset = client.get("C_BaseModelEntity", "m_vecViewOffset")?;
        offsets.pawn.eye_angles = client.get("C_CSPlayerPawn", "m_angEyeAngles")?;
        offsets.pawn.velocity = client.get("C_BaseEntity", "m_vecAbsVelocity")?;
        offsets.pawn.flags = client.get("C_BaseEntity", "m_fFlags")?;
        offsets.pawn.aim_punch_services = client.get("C_CSPlayerPawn", "m_pAimPunchServices")?;
        offsets.pawn.shots_fired = client.get("C_CSPlayerPawn", "m_iShotsFired")?;
        offsets.pawn.view_angles = client.get("C_BasePlayerPawn", "v_angle")?;
        offsets.pawn.spotted_state = client.get("C_CSPlayerPawn", "m_entitySpottedState")?;
        offsets.pawn.crosshair_entity = client.get("C_CSPlayerPawn", "m_iIDEntIndex")?;
        offsets.pawn.is_scoped = client.get("C_CSPlayerPawn", "m_bIsScoped")?;
        offsets.pawn.flash_duration = client.get("C_CSPlayerPawnBase", "m_flFlashDuration")?;
        offsets.pawn.flash_alpha = client.get("C_CSPlayerPawnBase", "m_flFlashMaxAlpha")?;
        offsets.pawn.deathmatch_immunity = client.get("C_CSPlayerPawn", "m_bGunGameImmunity")?;

        offsets.pawn.camera_services = client.get("C_BasePlayerPawn", "m_pCameraServices")?;
        offsets.pawn.item_services = client.get("C_BasePlayerPawn", "m_pItemServices")?;
        offsets.pawn.weapon_services = client.get("C_BasePlayerPawn", "m_pWeaponServices")?;
        offsets.pawn.movement_services = client.get("C_BasePlayerPawn", "m_pMovementServices")?;
        offsets.pawn.observer_services = client.get("C_BasePlayerPawn", "m_pObserverServices")?;

        offsets.game_scene_node.dormant = client.get("CGameSceneNode", "m_bDormant")?;
        offsets.game_scene_node.origin = client.get("CGameSceneNode", "m_vecAbsOrigin")?;
        offsets.game_scene_node.model_state = client.get("CSkeletonInstance", "m_modelState")?;

        offsets.skeleton.skeleton_instance =
            client.get("CBodyComponentSkeletonInstance", "m_skeletonInstance")?;

        offsets.molotov.is_incendiary = client.get("C_MolotovProjectile", "m_bIsIncGrenade")?;

        offsets.inferno.is_burning = client.get("C_Inferno", "m_bFireIsBurning")?;
        offsets.inferno.fire_count = client.get("C_Inferno", "m_fireCount")?;
        offsets.inferno.fire_positions = client.get("C_Inferno", "m_firePositions")?;

        offsets.spotted_state.mask =
            client.get_opt("EntitySpottedState_t", "m_bSpottedByMask").unwrap_or(0);

        offsets.action_tracking.round_kills = client.get(
            "CCSPlayerController_ActionTrackingServices",
            "m_iNumRoundKills",
        )?;
        offsets.action_tracking.round_damage = client.get(
            "CCSPlayerController_ActionTrackingServices",
            "m_flTotalRoundDamageDealt",
        )?;

        offsets.camera_services.fov = client.get("CCSPlayerBase_CameraServices", "m_iFOV")?;

        offsets.item_services.has_defuser =
            client.get("CCSPlayer_ItemServices", "m_bHasDefuser")?;
        offsets.item_services.has_helmet = client.get("CCSPlayer_ItemServices", "m_bHasHelmet")?;

        // Smoke offsets for no-smoke functionality
        offsets.smoke.did_smoke_effect = client.get("C_SmokeGrenadeProjectile", "m_bDidSmokeEffect")?;

        offsets.weapon_services.weapons = client.get("CPlayer_WeaponServices", "m_hMyWeapons")?;
        offsets.weapon_services.active_weapon = client.get("CPlayer_WeaponServices", "m_hActiveWeapon")?;

        offsets.observer_services.target =
            client.get("CPlayer_ObserverServices", "m_hObserverTarget")?;

        offsets.movement_services.force_on_mask =
            client.get_opt("CPlayer_MovementServices", "m_nButtonForceOnMask").unwrap_or(0);

        offsets.weapon.attribute_manager = client.get("C_EconEntity", "m_AttributeManager")?;
        offsets.weapon.item = client.get("C_AttributeContainer", "m_Item")?;
        offsets.weapon.item_definition_index =
            client.get("C_EconItemView", "m_iItemDefinitionIndex")?;

        offsets.planted_c4.is_ticking = client.get("C_PlantedC4", "m_bBombTicking")?;
        offsets.planted_c4.blow_time = client.get("C_PlantedC4", "m_flC4Blow")?;
        offsets.planted_c4.being_defused = client.get("C_PlantedC4", "m_bBeingDefused")?;
        offsets.planted_c4.is_defused = client.get("C_PlantedC4", "m_bBombDefused")?;
        offsets.planted_c4.has_exploded = client.get("C_PlantedC4", "m_bHasExploded")?;
        offsets.planted_c4.defuse_time_left = client.get("C_PlantedC4", "m_flDefuseCountDown")?;

        offsets.entity_identity.size = client.get_class("CEntityIdentity")?.size();

        log::debug!("offsets: {:?} ({:?})", offsets, Instant::now() - start);
        Some(offsets)
    }
}
