use super::Component;
#[derive(Clone, Copy, Debug, Default)]
pub struct DrawOrder(pub i32);

impl Component for DrawOrder {}