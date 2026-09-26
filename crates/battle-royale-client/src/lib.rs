#![forbid(unsafe_code)]

use battle_royale_core::MatchCommand;
use input_bindings_core::{
    ActionDefinition, ActionRegistry, DeviceClass, Provenance, RepeatPolicy,
};

pub const ACTION_MOVE: &str = "battle-royale.move";
pub const ACTION_LOOK: &str = "battle-royale.look";
pub const ACTION_MENU: &str = "battle-royale.menu";

pub const MOBILE_STICK_SCALE: i32 = 1_000;
pub const MOBILE_STICK_DEAD_ZONE: i32 = 420;
pub const MOBILE_DIAGONAL_ENTER_PER_MILLE: i32 = 600;
pub const MOBILE_DIAGONAL_EXIT_PER_MILLE: i32 = 450;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MobileStickSample {
    pub x: i32,
    pub y: i32,
}

impl MobileStickSample {
    pub const ZERO: Self = Self { x: 0, y: 0 };

    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MobileMovementIntent {
    pub x: i8,
    pub z: i8,
}

impl MobileMovementIntent {
    pub const STOPPED: Self = Self { x: 0, z: 0 };

    #[must_use]
    pub const fn command(self) -> MatchCommand {
        MatchCommand::SetMovement {
            x: self.x,
            z: self.z,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MobileMovementAdapter {
    current: MobileMovementIntent,
}

impl MobileMovementAdapter {
    #[must_use]
    pub const fn current(self) -> MobileMovementIntent {
        self.current
    }

    pub fn update(&mut self, sample: MobileStickSample) -> Option<MatchCommand> {
        let next = quantize_mobile_stick(sample, self.current);
        if next == self.current {
            return None;
        }

        self.current = next;
        Some(next.command())
    }

    pub fn release(&mut self) -> Option<MatchCommand> {
        self.update(MobileStickSample::ZERO)
    }
}

#[must_use]
pub fn battle_royale_action_registry() -> ActionRegistry {
    ActionRegistry {
        actions: vec![
            action(
                ACTION_MOVE,
                "Move",
                "Move the player. Touch thumbsticks are quantized into the same authoritative movement command as other clients.",
                vec![
                    DeviceClass::Keyboard,
                    DeviceClass::Gamepad,
                    DeviceClass::Pointer,
                ],
            ),
            action(
                ACTION_LOOK,
                "Look",
                "Aim or rotate the local camera. Look input is client-local until an authoritative gameplay command requires it.",
                vec![
                    DeviceClass::Mouse,
                    DeviceClass::Gamepad,
                    DeviceClass::Pointer,
                ],
            ),
            action(
                ACTION_MENU,
                "Menu",
                "Open or close local Battle Royale controls and settings.",
                vec![
                    DeviceClass::Keyboard,
                    DeviceClass::Gamepad,
                    DeviceClass::Pointer,
                ],
            ),
        ],
    }
}

#[must_use]
pub fn quantize_mobile_stick(
    sample: MobileStickSample,
    previous: MobileMovementIntent,
) -> MobileMovementIntent {
    let x = sample.x.clamp(-MOBILE_STICK_SCALE, MOBILE_STICK_SCALE);
    let screen_y = sample.y.clamp(-MOBILE_STICK_SCALE, MOBILE_STICK_SCALE);

    let radius_squared = (x * x) + (screen_y * screen_y);
    if radius_squared <= MOBILE_STICK_DEAD_ZONE * MOBILE_STICK_DEAD_ZONE {
        return MobileMovementIntent::STOPPED;
    }

    let z = -screen_y;
    let abs_x = x.abs();
    let abs_z = z.abs();

    let (x_active, z_active) = if abs_x >= abs_z {
        (
            true,
            secondary_axis_is_active(abs_z, abs_x, previous.z != 0),
        )
    } else {
        (
            secondary_axis_is_active(abs_x, abs_z, previous.x != 0),
            true,
        )
    };

    MobileMovementIntent {
        x: if x_active { axis_sign(x) } else { 0 },
        z: if z_active { axis_sign(z) } else { 0 },
    }
}

fn secondary_axis_is_active(secondary: i32, dominant: i32, was_active: bool) -> bool {
    if dominant == 0 {
        return false;
    }

    let threshold = if was_active {
        MOBILE_DIAGONAL_EXIT_PER_MILLE
    } else {
        MOBILE_DIAGONAL_ENTER_PER_MILLE
    };
    secondary * MOBILE_STICK_SCALE >= dominant * threshold
}

const fn axis_sign(value: i32) -> i8 {
    if value > 0 {
        1
    } else if value < 0 {
        -1
    } else {
        0
    }
}

fn action(
    id: &str,
    title: &str,
    description: &str,
    allowed_devices: Vec<DeviceClass>,
) -> ActionDefinition {
    ActionDefinition {
        id: id.to_owned(),
        title: title.to_owned(),
        description: Some(description.to_owned()),
        category_path: vec!["Battle Royale".to_owned()],
        repeat_policy: RepeatPolicy::Never,
        allowed_devices,
        defaults: Vec::new(),
        provenance: Some(Provenance {
            source: "battle-royale".to_owned(),
            version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use input_bindings_core::validate_registry;

    #[test]
    fn action_registry_is_valid_and_exposes_pointer_actions_for_mobile() {
        let registry = battle_royale_action_registry();
        let report = validate_registry(&registry, None);

        assert!(report.valid, "{:?}", report.diagnostics);
        assert_eq!(registry.actions.len(), 3);
        assert!(registry.actions.iter().any(|action| {
            action.id == ACTION_MOVE && action.allowed_devices.contains(&DeviceClass::Pointer)
        }));
        assert!(registry.actions.iter().any(|action| {
            action.id == ACTION_LOOK && action.allowed_devices.contains(&DeviceClass::Pointer)
        }));
    }

    #[test]
    fn radial_dead_zone_does_not_emit_movement() {
        let previous = MobileMovementIntent::STOPPED;

        assert_eq!(
            quantize_mobile_stick(MobileStickSample::new(200, -200), previous),
            MobileMovementIntent::STOPPED
        );
        assert_eq!(
            quantize_mobile_stick(MobileStickSample::new(420, 0), previous),
            MobileMovementIntent::STOPPED
        );
    }

    #[test]
    fn mobile_axes_map_to_existing_world_movement_axes() {
        assert_eq!(
            quantize_mobile_stick(
                MobileStickSample::new(MOBILE_STICK_SCALE, 0),
                MobileMovementIntent::STOPPED,
            ),
            MobileMovementIntent { x: 1, z: 0 }
        );
        assert_eq!(
            quantize_mobile_stick(
                MobileStickSample::new(0, -MOBILE_STICK_SCALE),
                MobileMovementIntent::STOPPED,
            ),
            MobileMovementIntent { x: 0, z: 1 }
        );
        assert_eq!(
            quantize_mobile_stick(
                MobileStickSample::new(-MOBILE_STICK_SCALE, MOBILE_STICK_SCALE),
                MobileMovementIntent::STOPPED,
            ),
            MobileMovementIntent { x: -1, z: -1 }
        );
    }

    #[test]
    fn diagonal_hysteresis_avoids_direction_chatter() {
        let cardinal = quantize_mobile_stick(
            MobileStickSample::new(1_000, -550),
            MobileMovementIntent::STOPPED,
        );
        assert_eq!(cardinal, MobileMovementIntent { x: 1, z: 0 });

        let diagonal = quantize_mobile_stick(MobileStickSample::new(1_000, -650), cardinal);
        assert_eq!(diagonal, MobileMovementIntent { x: 1, z: 1 });

        let retained = quantize_mobile_stick(MobileStickSample::new(1_000, -500), diagonal);
        assert_eq!(retained, MobileMovementIntent { x: 1, z: 1 });

        let exited = quantize_mobile_stick(MobileStickSample::new(1_000, -400), retained);
        assert_eq!(exited, MobileMovementIntent { x: 1, z: 0 });
    }

    #[test]
    fn adapter_only_emits_authoritative_commands_when_quantized_intent_changes() {
        let mut adapter = MobileMovementAdapter::default();

        assert_eq!(
            adapter.update(MobileStickSample::new(900, 0)),
            Some(MatchCommand::SetMovement { x: 1, z: 0 })
        );
        assert_eq!(adapter.update(MobileStickSample::new(700, 50)), None);
        assert_eq!(
            adapter.update(MobileStickSample::new(700, -700)),
            Some(MatchCommand::SetMovement { x: 1, z: 1 })
        );
        assert_eq!(
            adapter.release(),
            Some(MatchCommand::SetMovement { x: 0, z: 0 })
        );
        assert_eq!(adapter.release(), None);
    }

    #[test]
    fn samples_are_bounded_before_quantization() {
        assert_eq!(
            quantize_mobile_stick(
                MobileStickSample::new(50_000, -50_000),
                MobileMovementIntent::STOPPED,
            ),
            MobileMovementIntent { x: 1, z: 1 }
        );
    }
}
