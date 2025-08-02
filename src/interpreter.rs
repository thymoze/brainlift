use std::{
    cmp::min,
    io::{self, Read},
};

use crate::{
    cli::EofBehaviour,
    program::{
        Instruction::{self, *},
        Program,
    },
};

pub struct Interpreter {
    max_array_size: usize,
    eof_behaviour: EofBehaviour,
    state: State,
}

#[derive(Debug)]
struct State {
    array: Vec<u8>,
    pointer: usize,
}

impl Interpreter {
    pub fn new(max_array_size: usize, eof_behaviour: EofBehaviour) -> Self {
        Self {
            max_array_size,
            eof_behaviour,
            state: State {
                array: vec![0; 1],
                pointer: 0,
            },
        }
    }

    pub fn run(&mut self, program: &Program) {
        for instruction in &program.instructions {
            self.execute_instruction(instruction);
        }
    }

    fn execute_instruction(&mut self, instruction: &Instruction) {
        match instruction {
            Debug => {
                println!("{:?}", self.state);
            }
            Increment => self.increment(),
            Decrement => self.decrement(),
            Right => self.right(),
            Left => self.left(),
            Output => self.output(),
            Input => self.input(),
            Loop(instructions) => self.loop_(instructions),
        }
    }

    fn increment(&mut self) {
        *self.current() = self.current().wrapping_add(1)
    }

    fn decrement(&mut self) {
        *self.current() = self.current().wrapping_sub(1)
    }

    fn right(&mut self) {
        let index = self.state.pointer + 1;
        if index >= self.max_array_size {
            panic!("tried to move rightwards out-of-bounds");
        }

        // grow array if necessary and possible
        let current_size = self.state.array.len();
        if self.state.pointer == current_size - 1 && current_size < self.max_array_size {
            let new_size = min(self.max_array_size, current_size * 2);
            self.state.array.resize(new_size, 0);
        }

        self.state.pointer = index;
    }

    fn left(&mut self) {
        if self.state.pointer == 0 {
            panic!("tried to move leftwards out-of-bounds");
        }

        self.state.pointer -= 1;
    }

    fn output(&mut self) {
        print!("{}", *self.current() as char)
    }

    fn input(&mut self) {
        let input = io::stdin()
            .lock()
            .bytes()
            .next()
            .transpose()
            .expect("failed to read from stdin");

        if let Some(input) = input {
            *self.current() = input;
        } else {
            match self.eof_behaviour {
                EofBehaviour::Ignore => {}
                EofBehaviour::Zero => *self.current() = 0,
            }
        }
    }

    fn loop_(&mut self, instructions: &[Instruction]) {
        while *self.current() != 0 {
            for i in instructions {
                self.execute_instruction(i);
            }
        }
    }

    fn current(&mut self) -> &mut u8 {
        &mut self.state.array[self.state.pointer]
    }
}
