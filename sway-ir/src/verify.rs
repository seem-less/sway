//! Code to validate the IR in a [`Context`].
//!
//! During creation, deserialization and optimization the IR should be verified to be in a
//! consistent valid state, using the functions in this module.

mod instructions;
mod storage;

use crate::{
    block::BlockContent,
    context::Context,
    error::IrError,
    function::FunctionContent,
    module::ModuleContent,
    verify::{instructions::InstructionVerifier, storage::verify_storage},
};

impl Context {
    /// Verify the contents of this [`Context`] is valid.
    pub fn verify(self) -> Result<Self, Vec<IrError>> {
        let mut errors = Vec::new();

        for (_, module) in &self.modules {
            for function in &module.functions {
                let fn_content = &self.functions[function.0];

                // Firstly verify each instruction in each block.
                for block in &fn_content.blocks {
                    errors.extend(
                        self.verify_block(module, fn_content, &self.blocks[block.0])
                            .err()
                            .into_iter(),
                    );
                }

                // Secondly verify this function's storage behaviour matches its annotations.
                errors.extend(verify_storage(&self, function).err().into_iter());
            }
        }

        if errors.is_empty() {
            Ok(self)
        } else {
            Err(errors)
        }
    }

    fn verify_block(
        &self,
        module: &ModuleContent,
        function: &FunctionContent,
        block: &BlockContent,
    ) -> Result<(), IrError> {
        if block.instructions.len() <= 1 && block.num_predecessors(self) == 0 {
            // Empty (containing only the phi) unreferenced blocks are a harmless artefact.
            return Ok(());
        }

        InstructionVerifier::new(self, module, function, block).verify_instructions()?;

        let (last_is_term, num_terms) =
            block.instructions.iter().fold((false, 0), |(_, n), ins| {
                if ins.is_terminator(self) {
                    (true, n + 1)
                } else {
                    (false, n)
                }
            });
        if !last_is_term {
            Err(IrError::MissingTerminator(block.label.clone()))
        } else if num_terms != 1 {
            Err(IrError::MisplacedTerminator(block.label.clone()))
        } else {
            Ok(())
        }
    }
}
