use std::{fs::File, io::Write, path::PathBuf};

use cranelift::{
    codegen::ir::{BlockArg, FuncRef},
    prelude::*,
};
use cranelift_module::{FuncId, FuncOrDataId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};

use crate::{
    cli::EofBehaviour,
    program::{Instruction, Program},
};

const TARGET_TRIPLE: &str = "x86_64";
const ENTRYPOINT_FUNCTION_SYMBOL: &str = "main";
const GETCHAR_FUNCTION_SYMBOL: &str = "getchar";
const PUTCHAR_FUNCTION_SYMBOL: &str = "putchar";
const CALLOC_FUNCTION_SYMBOL: &str = "calloc";
const FREE_FUNCTION_SYMBOL: &str = "free";

pub struct Compiler {
    max_array_size: usize,
    eof_behaviour: EofBehaviour,
}

impl Compiler {
    pub fn new(max_array_size: usize, eof_behaviour: EofBehaviour) -> Self {
        Self {
            max_array_size,
            eof_behaviour,
        }
    }

    pub fn compile(mut self, program: &Program, output_file: PathBuf) {
        let isa = {
            let mut builder = settings::builder();
            builder.set("opt_level", "none").unwrap();
            builder.enable("is_pic").unwrap();
            let flags = settings::Flags::new(builder);
            isa::lookup_by_name(TARGET_TRIPLE)
                .unwrap()
                .finish(flags)
                .unwrap()
        };

        let mut module = {
            let translation_unit_name = output_file.file_stem().unwrap().as_encoded_bytes();
            let libcall_names = cranelift_module::default_libcall_names();
            let builder = ObjectBuilder::new(isa, translation_unit_name, libcall_names).unwrap();
            ObjectModule::new(builder)
        };

        let _main_declaration = {
            let sig = Signature {
                call_conv: module.isa().default_call_conv(),
                params: vec![],
                returns: vec![AbiParam::new(types::I32)],
            };

            module
                .declare_function(ENTRYPOINT_FUNCTION_SYMBOL, Linkage::Export, &sig)
                .unwrap()
        };

        self.declare_external_functions(&mut module);

        self.main_function(&mut module, program);

        let product = module.finish();

        {
            let bytes = product.emit().unwrap();

            let mut f = File::create(&output_file).unwrap();
            f.write_all(&bytes).unwrap();

            println!("finished compilation of {output_file:?}");
        }
    }

    fn main_function(&mut self, module: &mut ObjectModule, program: &Program) {
        let mut ctx = codegen::Context::new();
        let mut fctx = FunctionBuilderContext::new();

        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut fctx);
        builder.func.signature = Signature {
            call_conv: module.isa().default_call_conv(),
            params: vec![],
            returns: vec![AbiParam::new(types::I32)],
        };

        let block0 = builder.create_block();
        builder.switch_to_block(block0);
        builder.seal_block(block0);

        let calloc =
            module.declare_func_in_func(self.func_id(module, CALLOC_FUNCTION_SYMBOL), builder.func);
        let array_ptr = {
            let size_t = Type::int(module.target_config().pointer_bits() as u16).unwrap();

            let n = builder.ins().iconst(size_t, self.max_array_size as i64);
            let size = builder.ins().iconst(size_t, 1);
            let call = builder.ins().call(calloc, &[n, size]);
            builder.inst_results(call)[0]
        };

        let putchar = module
            .declare_func_in_func(self.func_id(module, PUTCHAR_FUNCTION_SYMBOL), builder.func);
        let getchar = module
            .declare_func_in_func(self.func_id(module, GETCHAR_FUNCTION_SYMBOL), builder.func);

        let mut e = Emitter {
            module,
            builder: &mut builder,
            putchar,
            getchar,
            array_ptr,
            eof_behaviour: self.eof_behaviour,
        };

        for instruction in &program.instructions {
            e.emit(instruction);
        }

        let free =
            module.declare_func_in_func(self.func_id(module, FREE_FUNCTION_SYMBOL), builder.func);
        builder.ins().call(free, &[array_ptr]);

        let zero = builder.ins().iconst(types::I32, 0);
        builder.ins().return_(&[zero]);

        codegen::verify_function(builder.func, module.isa()).expect("verification should succeed");

        builder.finalize();

        module
            .define_function(self.func_id(module, ENTRYPOINT_FUNCTION_SYMBOL), &mut ctx)
            .unwrap();

        // println!("fn {ENTRYPOINT_FUNCTION_SYMBOL}:\n{}", &ctx.func);

        ctx.clear();
    }

    fn declare_external_functions(&mut self, module: &mut ObjectModule) {
        let _putchar_declaration = {
            let sig = Signature {
                params: vec![AbiParam::new(types::I32)],
                returns: vec![AbiParam::new(types::I32)],
                call_conv: module.isa().default_call_conv(),
            };

            module
                .declare_function(PUTCHAR_FUNCTION_SYMBOL, Linkage::Import, &sig)
                .unwrap()
        };

        let _getchar_declaration = {
            let sig = Signature {
                params: vec![],
                returns: vec![AbiParam::new(types::I32)],
                call_conv: module.isa().default_call_conv(),
            };

            module
                .declare_function(GETCHAR_FUNCTION_SYMBOL, Linkage::Import, &sig)
                .unwrap()
        };

        let size_t = Type::int(module.target_config().pointer_bits() as u16).unwrap();
        let ptr_t = module.target_config().pointer_type();

        let _calloc_declaration = {
            let sig = Signature {
                params: vec![AbiParam::new(size_t), AbiParam::new(size_t)],
                returns: vec![AbiParam::new(ptr_t)],
                call_conv: module.isa().default_call_conv(),
            };

            module
                .declare_function(CALLOC_FUNCTION_SYMBOL, Linkage::Import, &sig)
                .unwrap()
        };

        let _free_declaration = {
            let sig = Signature {
                params: vec![AbiParam::new(ptr_t)],
                returns: vec![],
                call_conv: module.isa().default_call_conv(),
            };

            module
                .declare_function(FREE_FUNCTION_SYMBOL, Linkage::Import, &sig)
                .unwrap()
        };
    }

    fn func_id(&self, module: &ObjectModule, name: &str) -> FuncId {
        let Some(FuncOrDataId::Func(func_id)) = module.get_name(name) else {
            panic!("{name} should be declared")
        };
        func_id
    }
}

struct Emitter<'a, 'b> {
    module: &'a mut ObjectModule,
    builder: &'a mut FunctionBuilder<'b>,
    putchar: FuncRef,
    getchar: FuncRef,
    array_ptr: Value,
    eof_behaviour: EofBehaviour,
}

impl<'a, 'b> Emitter<'a, 'b> {
    pub fn emit(&mut self, instruction: &Instruction) {
        let size_t = Type::int(self.module.target_config().pointer_bits() as u16).unwrap();

        match instruction {
            Instruction::Debug => {}
            Instruction::Increment => {
                let val = self
                    .builder
                    .ins()
                    .load(types::I8, MemFlags::new(), self.array_ptr, 0);
                let new_val = self.builder.ins().iadd_imm(val, 1);

                self.builder
                    .ins()
                    .store(MemFlags::new(), new_val, self.array_ptr, 0);
            }
            Instruction::Decrement => {
                let val = self
                    .builder
                    .ins()
                    .load(types::I8, MemFlags::new(), self.array_ptr, 0);
                let new_val = self.builder.ins().iadd_imm(val, -1);

                self.builder
                    .ins()
                    .store(MemFlags::new(), new_val, self.array_ptr, 0);
            }
            Instruction::Right => {
                self.array_ptr = self.builder.ins().iadd_imm(self.array_ptr, 1);
            }
            Instruction::Left => {
                self.array_ptr = self.builder.ins().iadd_imm(self.array_ptr, -1);
            }
            Instruction::Output => {
                let val = self
                    .builder
                    .ins()
                    .sload8(types::I32, MemFlags::new(), self.array_ptr, 0);
                self.builder.ins().call(self.putchar, &[val]);
            }
            Instruction::Input => {
                let inst = self.builder.ins().call(self.getchar, &[]);
                let val = self.builder.inst_results(inst)[0];

                let eof_block = self.builder.create_block();
                self.builder.append_block_param(eof_block, size_t);
                let store_block = self.builder.create_block();
                self.builder.append_block_param(store_block, size_t);
                self.builder.append_block_param(store_block, types::I32);

                let eof = self.builder.ins().iconst(types::I32, -1);
                let is_eof = self.builder.ins().icmp(IntCC::Equal, val, eof);
                self.builder.ins().brif(
                    is_eof,
                    eof_block,
                    &[BlockArg::Value(self.array_ptr)],
                    store_block,
                    &[BlockArg::Value(self.array_ptr), BlockArg::Value(val)],
                );

                self.builder.seal_block(eof_block);
                self.builder.switch_to_block(eof_block);

                let next_block = self.builder.create_block();
                self.builder.append_block_param(next_block, size_t);

                self.array_ptr = self
                    .builder
                    .block_params(self.builder.current_block().unwrap())[0];
                match self.eof_behaviour {
                    EofBehaviour::Ignore => {
                        self.builder
                            .ins()
                            .jump(next_block, &[BlockArg::Value(self.array_ptr)]);
                    }
                    EofBehaviour::Zero => {
                        let zero = self.builder.ins().iconst(types::I32, 0);
                        self.builder.ins().jump(
                            store_block,
                            &[BlockArg::Value(self.array_ptr), BlockArg::Value(zero)],
                        );
                    }
                }

                self.builder.seal_block(store_block);

                self.builder.switch_to_block(store_block);
                let block_params = self
                    .builder
                    .block_params(self.builder.current_block().unwrap());
                self.array_ptr = block_params[0];
                let val = block_params[1];

                self.builder
                    .ins()
                    .istore8(MemFlags::new(), val, self.array_ptr, 0);
                self.builder
                    .ins()
                    .jump(next_block, &[BlockArg::Value(self.array_ptr)]);

                self.builder.seal_block(next_block);
                self.builder.switch_to_block(next_block);
                self.array_ptr = self
                    .builder
                    .block_params(self.builder.current_block().unwrap())[0];
            }
            Instruction::Loop(instructions) => {
                let loop_test_block = self.builder.create_block();
                self.builder.append_block_param(loop_test_block, size_t);
                self.builder
                    .ins()
                    .jump(loop_test_block, &[BlockArg::Value(self.array_ptr)]);
                self.builder.switch_to_block(loop_test_block);

                let then_block = self.builder.create_block();
                self.builder.append_block_param(then_block, size_t);
                let else_block = self.builder.create_block();
                self.builder.append_block_param(else_block, size_t);

                self.array_ptr = self
                    .builder
                    .block_params(self.builder.current_block().unwrap())[0];
                let val = self
                    .builder
                    .ins()
                    .load(types::I8, MemFlags::new(), self.array_ptr, 0);
                self.builder.ins().brif(
                    val,
                    then_block,
                    &[BlockArg::Value(self.array_ptr)],
                    else_block,
                    &[BlockArg::Value(self.array_ptr)],
                );

                self.builder.seal_block(then_block);
                self.builder.seal_block(else_block);
                self.builder.switch_to_block(then_block);
                self.array_ptr = self
                    .builder
                    .block_params(self.builder.current_block().unwrap())[0];

                for i in instructions {
                    self.emit(i);
                }

                self.builder
                    .ins()
                    .jump(loop_test_block, &[BlockArg::Value(self.array_ptr)]);

                self.builder.seal_block(loop_test_block);
                self.builder.switch_to_block(else_block);
                self.array_ptr = self
                    .builder
                    .block_params(self.builder.current_block().unwrap())[0];
            }
        }
    }
}
