use std::collections::HashSet;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum CameraAction {
    MoveForward,
    MoveBackward,
    MoveLeft,
    MoveRight,
}

pub struct CameraActionState {
    pub actions: HashSet<CameraAction>,
}

impl CameraActionState {
    pub fn is_active(&self, action: CameraAction) -> bool {
        self.actions.contains(&action)
    }
}
