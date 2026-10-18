use std::collections::HashSet;

#[allow(dead_code)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Action {
    MoveForward,
    MoveBackward,
    MoveLeft,
    MoveRight,
    ShowGizmos,
}

#[allow(dead_code)]
pub struct CameraActionState {
    pub actions: HashSet<Action>,
}

#[allow(dead_code)]
impl CameraActionState {

    pub fn is_active(&self, action: Action) -> bool {
        self.actions.contains(&action)
    }
}
