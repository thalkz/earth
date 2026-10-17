use std::collections::HashSet;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Action {
    MoveForward,
    MoveBackward,
    MoveLeft,
    MoveRight,
    ShowGizmos,
}

pub struct CameraActionState {
    pub actions: HashSet<Action>,
}

impl CameraActionState {
    pub fn is_active(&self, action: Action) -> bool {
        self.actions.contains(&action)
    }
}
