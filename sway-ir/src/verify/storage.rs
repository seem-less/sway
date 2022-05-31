///! Verification that:
///!   a) if a function performs a storage instruction then it has the appropriate storage
///!   attribute, and
///!   b) if a function has a storage attribute then it does actually perform a storage operation.
use crate::{
    context::Context,
    error::IrError,
    function::Function,
    instruction::Instruction,
    metadata::{Metadatum, StorageOperation},
    value::ValueDatum,
};
use sway_types::span::Span;

pub(super) fn verify_storage(context: &Context, function: &Function) -> Result<(), IrError> {
    // Iterate for each instruction in the function and if it's an ASM block then iterate for each
    // instruction. Gather whether we have read or write storage operations based on the existence
    // of the storage opcodes.
    let (reads, writes) = function
        .instruction_iter(context)
        .filter_map(|(_block, ins_value)| {
            let ins = &context.values[ins_value.0].value;
            if let ValueDatum::Instruction(Instruction::AsmBlock(asm_block, _args)) = ins {
                Some(&context.asm_blocks[asm_block.0])
            } else {
                None
            }
        })
        .flat_map(|asm_block| asm_block.body.iter())
        .fold((false, false), |(reads, writes), asm_op| {
            match asm_op.name.as_str() {
                "srw" | "srwq" => (true, writes),
                "sww" | "swwq" => (reads, true),
                _ => (reads, writes),
            }
        });

    let function = &context.functions[function.0];
    let attributed_purity = function
        .storage_md_idx
        .map(|idx| match &context.metadata[idx.0] {
            Metadatum::StorageAttribute(storage_op) => Some(storage_op),
            _otherwise => None,
        })
        .flatten();
    let span = function
        .span_md_idx
        .map(|idx| idx.to_span(context).ok())
        .flatten()
        .unwrap_or_else(|| Span::dummy());

    // Simple macros for each of the error types, which also grab `span`.
    // mk_error!(missing, Reads) --> Err(IrError::StorageMissingAttribute(StorageOperation::Reads, span))
    macro_rules! mk_err_helper {
        ($err_var:ident, $op:ident) => {
            Err(IrError::$err_var(StorageOperation::$op, span))
        };
    }
    macro_rules! mk_err {
        (missing, $op:ident) => {
            mk_err_helper!(StorageMissingAttribute, $op)
        };
        (mismatched, $op:ident) => {
            mk_err_helper!(StorageMismatchedAttribute, $op)
        };
        (unneeded, $op:ident) => {
            mk_err_helper!(StorageUnneededAttribute, $op)
        };
    }

    match (attributed_purity, reads, writes) {
        // Has no attributes but needs some.
        (None, true, false) => mk_err!(missing, Reads),
        (None, false, true) => mk_err!(missing, Writes),
        (None, true, true) => mk_err!(missing, ReadsWrites),

        // Or the attribute must match the behaviour.
        (Some(StorageOperation::Reads), false, true) => mk_err!(mismatched, Writes),
        (Some(StorageOperation::Writes), true, false) => mk_err!(mismatched, Reads),
        (Some(StorageOperation::Reads), _, true) => mk_err!(mismatched, ReadsWrites),
        (Some(StorageOperation::Writes), true, _) => mk_err!(mismatched, ReadsWrites),

        // Or we have unneeded attributes.
        (Some(StorageOperation::ReadsWrites), false, true) => mk_err!(unneeded, Reads),
        (Some(StorageOperation::ReadsWrites), true, false) => mk_err!(unneeded, Writes),
        (Some(unused_op), false, false) => Err(IrError::StorageUnneededAttribute(*unused_op, span)),

        // (Pure, false, false) is OK, as is (ReadsWrites, true, true).
        _otherwise => Ok(()),
    }
}
