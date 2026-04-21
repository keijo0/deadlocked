#![allow(unused)]
use serde::{Deserialize, Serialize};
use strum::EnumIter;

#[derive(Debug, Clone, Copy, EnumIter, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Bones {
    Hip = 0,
    Spine1 = 1,
    Spine2 = 2,
    Spine3 = 3,
    Spine4 = 4,
    Neck = 5,
    Head = 6,
    LeftShoulder = 8,
    LeftElbow = 9,
    LeftHand = 10,
    RightShoulder = 13,
    RightElbow = 14,
    RightHand = 15,
    LeftHip = 22,
    LeftKnee = 23,
    LeftFoot = 24,
    RightHip = 25,
    RightKnee = 26,
    RightFoot = 27,
}

impl Bones {
    pub const CONNECTIONS: [(Self, Self); 18] = [
        // spine
        (Self::Hip, Self::Spine1),
        (Self::Spine1, Self::Spine2),
        (Self::Spine2, Self::Spine3),
        (Self::Spine3, Self::Spine4),
        (Self::Spine4, Self::Neck),
        (Self::Neck, Self::Head),
        // left arm
        (Self::Neck, Self::LeftShoulder),
        (Self::LeftShoulder, Self::LeftElbow),
        (Self::LeftElbow, Self::LeftHand),
        // right arm
        (Self::Neck, Self::RightShoulder),
        (Self::RightShoulder, Self::RightElbow),
        (Self::RightElbow, Self::RightHand),
        // left leg
        (Self::Hip, Self::LeftHip),
        (Self::LeftHip, Self::LeftKnee),
        (Self::LeftKnee, Self::LeftFoot),
        // right leg
        (Self::Hip, Self::RightHip),
        (Self::RightHip, Self::RightKnee),
        (Self::RightKnee, Self::RightFoot),
    ];

    pub fn u64(self) -> u64 {
        self as u64
    }
}
