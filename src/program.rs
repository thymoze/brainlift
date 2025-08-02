#[derive(Debug)]
pub struct Program {
    pub instructions: Vec<Instruction>,
}

#[derive(Debug)]
pub enum Instruction {
    Increment,
    Decrement,
    Right,
    Left,
    Output,
    Input,
    Loop(Vec<Instruction>),
    Debug,
}
