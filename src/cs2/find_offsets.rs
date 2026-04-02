use std::time::Instant;

use utils::log;

use crate::{
    constants::cs2,
    cs2::{CS2, offsets::Offsets, schema::Schema},
};

impl CS2 {
    pub fn find_offsets(&self) -> Option<Offsets> {
        let start = Instant::now();
        let mut offsets = Offsets::default();

        offsets.library.client = self.process.module_base_address(cs2::CLIENT_LIB)?;
        offsets.library.engine = self.process.module_base_address(cs2::ENGINE_LIB)?;
        offsets.library.tier0 = self.process.module_base_address(cs2::TIER0_LIB)?;
        offsets.library.input = self.process.module_base_address(cs2::INPUT_LIB)?;
        offsets.library.sdl = self.process.module_base_address(cs2::SDL_LIB)?;
        offsets.library.schema = self.process.module_base_address(cs2::SCHEMA_LIB)?;

        let Some(resource_offset) = self
            .process
            .get_interface_offset(offsets.library.engine, "GameResourceServiceClientV0")
        else {
            log::warn!("could not get offset for GameResourceServiceClient");
            return None;
        };
        offsets.interface.resource = resource_offset;

        offsets.interface.entity =
            self.process.read::<u64>(offsets.interface.resource + 0x50) + 0x10;

        let Some(cvar_address) = self
            .process
            .get_interface_offset(offsets.library.tier0, "VEngineCvar0")
        else {
            log::warn!("could not get convar interface offset");
            return None;
        };
        offsets.interface.cvar = cvar_address;
        let Some(input_address) = self
            .process
            .get_interface_offset(offsets.library.input, "InputSystemVersion0")
        else {
            log::warn!("could not get input interface offset");
            return None;
        };
        offsets.interface.input = input_address;

        let Some(local_player) = self
            .process
            .scan("48 83 3D ? ? ? ? 00 0F 95 C0 C3", offsets.library.client)
        else {
            log::warn!("could not find local player offset");
            return None;
        };
        offsets.direct.local_player = self.process.get_relative_address(local_player, 0x03, 0x08);
        offsets.direct.button_state = self.process.read::<u32>(
            self.process
                .get_interface_function(offsets.interface.input, 19)
                + 0x14,
        ) as u64;

        let Some(view_matrix) = self
            .process
            .scan("C6 83 ? ? 00 00 01 4C 8D 05", offsets.library.client)
        else {
            log::warn!("could not find view matrix offset");
            return None;
        };

        offsets.direct.view_matrix =
            self.process
                .get_relative_address(view_matrix + 0x0A, 0x0, 0x04);

        let Some(sdl_window) = self
            .process
            .get_module_export(offsets.library.sdl, "SDL_GetKeyboardFocus")
        else {
            log::warn!("could not find sdl window offset");
            return None;
        };
        let sdl_window = self.process.get_relative_address(sdl_window, 0x02, 0x06);
        let sdl_window = self.process.read(sdl_window);
        offsets.direct.sdl_window = self.process.get_relative_address(sdl_window, 0x03, 0x07);

        let Some(planted_c4) = self.process.scan(
            "48 8D 35 ? ? ? ? 66 0F EF C0 C6 05 ? ? ? ? 01 48 8D 3D",
            offsets.library.client,
        ) else {
            log::warn!("could not find planted c4 offset");
            return None;
        };
        offsets.direct.planted_c4 = self.process.get_relative_address(planted_c4, 0x03, 0x0E);

        // xref "lobby_mapveto"
        let Some(global_vars) = self.process.scan(
            "48 8D 05 ? ? ? ? 48 8B 00 8B 48 ? E9",
            offsets.library.client,
        ) else {
            log::warn!("could not find global vars offset");
            return None;
        };
        offsets.direct.global_vars = self.process.get_relative_address(global_vars, 0x03, 0x07);

        let Some(ffa_address) = self
            .process
            .get_convar(offsets.interface.cvar, "mp_teammates_are_enemies")
        else {
            log::warn!("could not get mp_tammates_are_enemies convar offset");
            return None;
        };
        offsets.convar.ffa = ffa_address;
        let Some(sensitivity_address) = self
            .process
            .get_convar(offsets.interface.cvar, "sensitivity")
        else {
            log::warn!("could not get sensitivity convar offset");
            return None;
        };
        offsets.convar.sensitivity = sensitivity_address;

        let schema = Schema::new(&self.process, offsets.library.schema)?;
        let client = schema.get_library(cs2::CLIENT_LIB)?;

        // m_steamID may appear on different controller classes depending on CS2 version.
        // Try CBasePlayerController first (most common), then the CS-specific controller.
        offsets.controller.steam_id = client
            .get_opt("CBasePlayerController", "m_steamID")
            .or_else(|| client.get_opt("CCSPlayerController", "m_steamID"))
            .unwrap_or(0);
        // m_iszPlayerName and m_hPawn may appear on different controller classes depending on CS2 version.
        offsets.controller.name = client
            .get_opt("CBasePlayerController", "m_iszPlayerName")
            .or_else(|| client.get_opt("CCSPlayerController", "m_iszPlayerName"))
            .unwrap_or(0);
        offsets.controller.pawn = client
            .get_opt("CBasePlayerController", "m_hPawn")
            .or_else(|| client.get_opt("CCSPlayerController", "m_hPawn"))
            .unwrap_or(0);
        // m_hOwnerEntity may appear on different entity classes depending on CS2 version.
        offsets.controller.owner_entity = client
            .get_opt("C_BaseEntity", "m_hOwnerEntity")
            .or_else(|| client.get_opt("C_BaseModelEntity", "m_hOwnerEntity"))
            .unwrap_or(0);
        // m_iCompTeammateColor may appear on different controller classes depending on CS2 version.
        offsets.controller.color = client
            .get_opt("CCSPlayerController", "m_iCompTeammateColor")
            .or_else(|| client.get_opt("CBasePlayerController", "m_iCompTeammateColor"))
            .unwrap_or(0);
        // m_pActionTrackingServices may appear on different controller classes depending on CS2 version.
        offsets.controller.action_tracking_services = client
            .get_opt("CCSPlayerController", "m_pActionTrackingServices")
            .or_else(|| client.get_opt("CBasePlayerController", "m_pActionTrackingServices"))
            .unwrap_or(0);
        // m_nPing may appear on different controller classes depending on CS2 version.
        // Try CBasePlayerController first (most common), then the CS-specific controller,
        // and fall back to the legacy m_iPing name if m_nPing is absent.
        offsets.controller.ping = client
            .get_opt("CBasePlayerController", "m_nPing")
            .or_else(|| client.get_opt("CCSPlayerController", "m_nPing"))
            .or_else(|| client.get_opt("CBasePlayerController", "m_iPing"))
            .or_else(|| client.get_opt("CCSPlayerController", "m_iPing"))
            .unwrap_or(0);

        // C_BaseEntity fields may migrate to C_BaseModelEntity between CS2 versions.
        offsets.pawn.health = client
            .get_opt("C_BaseEntity", "m_iHealth")
            .or_else(|| client.get_opt("C_BaseModelEntity", "m_iHealth"))
            .unwrap_or(0);
        offsets.pawn.team = client
            .get_opt("C_BaseEntity", "m_iTeamNum")
            .or_else(|| client.get_opt("C_BaseModelEntity", "m_iTeamNum"))
            .unwrap_or(0);
        offsets.pawn.life_state = client
            .get_opt("C_BaseEntity", "m_lifeState")
            .or_else(|| client.get_opt("C_BaseModelEntity", "m_lifeState"))
            .unwrap_or(0);
        offsets.pawn.game_scene_node = client
            .get_opt("C_BaseEntity", "m_pGameSceneNode")
            .or_else(|| client.get_opt("C_BaseModelEntity", "m_pGameSceneNode"))
            .unwrap_or(0);
        offsets.pawn.velocity = client
            .get_opt("C_BaseEntity", "m_vecAbsVelocity")
            .or_else(|| client.get_opt("C_BaseModelEntity", "m_vecAbsVelocity"))
            .unwrap_or(0);
        offsets.pawn.flags = client
            .get_opt("C_BaseEntity", "m_fFlags")
            .or_else(|| client.get_opt("C_BaseModelEntity", "m_fFlags"))
            .unwrap_or(0);

        // C_BaseModelEntity fields may migrate to C_BaseEntity between CS2 versions.
        offsets.pawn.eye_offset = client
            .get_opt("C_BaseModelEntity", "m_vecViewOffset")
            .or_else(|| client.get_opt("C_BaseEntity", "m_vecViewOffset"))
            .unwrap_or(0);

        // C_CSPlayerPawn fields may migrate to C_CSPlayerPawnBase between CS2 versions.
        offsets.pawn.armor = client
            .get_opt("C_CSPlayerPawn", "m_ArmorValue")
            .or_else(|| client.get_opt("C_CSPlayerPawnBase", "m_ArmorValue"))
            .unwrap_or(0);
        offsets.pawn.weapon = client
            .get_opt("C_CSPlayerPawn", "m_pClippingWeapon")
            .or_else(|| client.get_opt("C_CSPlayerPawnBase", "m_pClippingWeapon"))
            .unwrap_or(0);
        offsets.pawn.eye_angles = client
            .get_opt("C_CSPlayerPawn", "m_angEyeAngles")
            .or_else(|| client.get_opt("C_CSPlayerPawnBase", "m_angEyeAngles"))
            .unwrap_or(0);
        offsets.pawn.aim_punch_cache = client
            .get_opt("C_CSPlayerPawn", "m_aimPunchTickFraction")
            .or_else(|| client.get_opt("C_CSPlayerPawnBase", "m_aimPunchTickFraction"))
            .map(|o| o + 8)
            .unwrap_or(0);
        offsets.pawn.shots_fired = client
            .get_opt("C_CSPlayerPawn", "m_iShotsFired")
            .or_else(|| client.get_opt("C_CSPlayerPawnBase", "m_iShotsFired"))
            .unwrap_or(0);
        offsets.pawn.spotted_state = client
            .get_opt("C_CSPlayerPawn", "m_entitySpottedState")
            .or_else(|| client.get_opt("C_CSPlayerPawnBase", "m_entitySpottedState"))
            .unwrap_or(0);
        offsets.pawn.crosshair_entity = client
            .get_opt("C_CSPlayerPawn", "m_iIDEntIndex")
            .or_else(|| client.get_opt("C_CSPlayerPawnBase", "m_iIDEntIndex"))
            .unwrap_or(0);
        offsets.pawn.is_scoped = client
            .get_opt("C_CSPlayerPawn", "m_bIsScoped")
            .or_else(|| client.get_opt("C_CSPlayerPawnBase", "m_bIsScoped"))
            .unwrap_or(0);
        offsets.pawn.deathmatch_immunity = client
            .get_opt("C_CSPlayerPawn", "m_bGunGameImmunity")
            .or_else(|| client.get_opt("C_CSPlayerPawnBase", "m_bGunGameImmunity"))
            .unwrap_or(0);

        // C_CSPlayerPawnBase fields may migrate to C_CSPlayerPawn between CS2 versions.
        offsets.pawn.flash_duration = client
            .get_opt("C_CSPlayerPawnBase", "m_flFlashDuration")
            .or_else(|| client.get_opt("C_CSPlayerPawn", "m_flFlashDuration"))
            .unwrap_or(0);

        // C_BasePlayerPawn fields may migrate to C_CSPlayerPawn between CS2 versions.
        offsets.pawn.fov_multiplier = client
            .get_opt("C_BasePlayerPawn", "m_flFOVSensitivityAdjust")
            .or_else(|| client.get_opt("C_CSPlayerPawn", "m_flFOVSensitivityAdjust"))
            .unwrap_or(0);
        offsets.pawn.view_angles = client
            .get_opt("C_BasePlayerPawn", "v_angle")
            .or_else(|| client.get_opt("C_CSPlayerPawn", "v_angle"))
            .unwrap_or(0);
        offsets.pawn.camera_services = client
            .get_opt("C_BasePlayerPawn", "m_pCameraServices")
            .or_else(|| client.get_opt("C_CSPlayerPawn", "m_pCameraServices"))
            .unwrap_or(0);
        offsets.pawn.item_services = client
            .get_opt("C_BasePlayerPawn", "m_pItemServices")
            .or_else(|| client.get_opt("C_CSPlayerPawn", "m_pItemServices"))
            .unwrap_or(0);
        offsets.pawn.weapon_services = client
            .get_opt("C_BasePlayerPawn", "m_pWeaponServices")
            .or_else(|| client.get_opt("C_CSPlayerPawn", "m_pWeaponServices"))
            .unwrap_or(0);
        offsets.pawn.observer_services = client
            .get_opt("C_BasePlayerPawn", "m_pObserverServices")
            .or_else(|| client.get_opt("C_CSPlayerPawn", "m_pObserverServices"))
            .unwrap_or(0);

        offsets.game_scene_node.dormant = client
            .get_opt("CGameSceneNode", "m_bDormant")
            .unwrap_or(0);
        offsets.game_scene_node.origin = client
            .get_opt("CGameSceneNode", "m_vecAbsOrigin")
            .unwrap_or(0);
        offsets.game_scene_node.model_state = client
            .get_opt("CSkeletonInstance", "m_modelState")
            .unwrap_or(0);

        offsets.skeleton.skeleton_instance = client
            .get_opt("CBodyComponentSkeletonInstance", "m_skeletonInstance")
            .unwrap_or(0);

        offsets.molotov.is_incendiary = client
            .get_opt("C_MolotovProjectile", "m_bIsIncGrenade")
            .unwrap_or(0);

        offsets.inferno.is_burning = client
            .get_opt("C_Inferno", "m_bFireIsBurning")
            .unwrap_or(0);
        offsets.inferno.fire_count = client
            .get_opt("C_Inferno", "m_fireCount")
            .unwrap_or(0);
        offsets.inferno.fire_positions = client
            .get_opt("C_Inferno", "m_firePositions")
            .unwrap_or(0);

        offsets.spotted_state.mask =
            client.get_opt("EntitySpottedState_t", "m_bSpottedByMask").unwrap_or(0);

        offsets.action_tracking.round_kills = client
            .get_opt("CCSPlayerController_ActionTrackingServices", "m_iNumRoundKills")
            .unwrap_or(0);
        offsets.action_tracking.round_damage = client
            .get_opt(
                "CCSPlayerController_ActionTrackingServices",
                "m_flTotalRoundDamageDealt",
            )
            .unwrap_or(0);

        offsets.camera_services.fov = client
            .get_opt("CCSPlayerBase_CameraServices", "m_iFOV")
            .unwrap_or(0);

        offsets.item_services.has_defuser = client
            .get_opt("CCSPlayer_ItemServices", "m_bHasDefuser")
            .unwrap_or(0);
        offsets.item_services.has_helmet = client
            .get_opt("CCSPlayer_ItemServices", "m_bHasHelmet")
            .unwrap_or(0);

        offsets.weapon_services.weapons = client
            .get_opt("CPlayer_WeaponServices", "m_hMyWeapons")
            .unwrap_or(0);

        offsets.observer_services.target = client
            .get_opt("CPlayer_ObserverServices", "m_hObserverTarget")
            .unwrap_or(0);

        offsets.weapon.attribute_manager = client
            .get_opt("C_EconEntity", "m_AttributeManager")
            .unwrap_or(0);
        offsets.weapon.item = client
            .get_opt("C_AttributeContainer", "m_Item")
            .unwrap_or(0);
        offsets.weapon.item_definition_index = client
            .get_opt("C_EconItemView", "m_iItemDefinitionIndex")
            .unwrap_or(0);

        offsets.planted_c4.is_ticking = client
            .get_opt("C_PlantedC4", "m_bBombTicking")
            .unwrap_or(0);
        offsets.planted_c4.blow_time = client
            .get_opt("C_PlantedC4", "m_flC4Blow")
            .unwrap_or(0);
        offsets.planted_c4.being_defused = client
            .get_opt("C_PlantedC4", "m_bBeingDefused")
            .unwrap_or(0);
        offsets.planted_c4.is_defused = client
            .get_opt("C_PlantedC4", "m_bBombDefused")
            .unwrap_or(0);
        offsets.planted_c4.has_exploded = client
            .get_opt("C_PlantedC4", "m_bHasExploded")
            .unwrap_or(0);
        offsets.planted_c4.defuse_time_left = client
            .get_opt("C_PlantedC4", "m_flDefuseCountDown")
            .unwrap_or(0);

        offsets.entity_identity.size = client.get_class("CEntityIdentity")?.size();

        log::debug!("offsets: {:?} ({:?})", offsets, Instant::now() - start);
        Some(offsets)
    }
}
