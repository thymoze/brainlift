use crate::program::{Instruction, Program};

pub struct Parser<'a> {
    source: &'a [u8],
    index: usize,
}

const INSTRUCTIONS: [u8; 9] = [b'+', b'-', b'>', b'<', b'.', b',', b'[', b']', b'#'];

impl<'a> Parser<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source: source.as_bytes(),
            index: 0,
        }
    }

    pub fn parse(&mut self) -> Result<Program, ParserError> {
        let mut instructions = Vec::new();
        while !self.is_at_end() {
            match self.instruction() {
                Ok(inst) => instructions.push(inst),
                Err(e) => return Err(e),
            }
            self.advance();
        }

        Ok(Program { instructions })
    }

    fn instruction(&mut self) -> Result<Instruction, ParserError> {
        match self.current() {
            b'#' => Ok(Instruction::Debug),
            b'+' => Ok(Instruction::Increment),
            b'-' => Ok(Instruction::Decrement),
            b'>' => Ok(Instruction::Right),
            b'<' => Ok(Instruction::Left),
            b'.' => Ok(Instruction::Output),
            b',' => Ok(Instruction::Input),
            b'[' => {
                let index = self.index;
                self.advance();
                let mut nested = Vec::new();
                while !self.is_at_end() && self.current() != b']' {
                    let inst = self.instruction()?;
                    nested.push(inst);

                    self.advance();
                }
                if self.current() != b']' {
                    let line = self.line_number(index);
                    return Err(ParserError::MismatchedBracket(line));
                }
                Ok(Instruction::Loop(nested))
            }
            _ => unreachable!(),
        }
    }

    fn current(&self) -> u8 {
        if self.is_at_end() {
            b'\0'
        } else {
            self.source[self.index]
        }
    }

    fn advance(&mut self) {
        loop {
            if self.is_at_end() {
                break;
            }

            self.index += 1;

            if INSTRUCTIONS.contains(&self.current()) {
                break;
            }
        }
    }

    fn is_at_end(&self) -> bool {
        self.index >= self.source.len()
    }

    fn line_number(&self, index: usize) -> usize {
        self.source[..index].iter().filter(|&&c| c == b'\n').count() + 1
    }
}

#[derive(Debug)]
pub enum ParserError {
    MismatchedBracket(usize),
}

impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParserError::MismatchedBracket(line) => {
                write!(f, "mismatched bracket in line {line}")
            }
        }
    }
}

impl std::error::Error for ParserError {}
