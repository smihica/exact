use crate::{LowerError, Lowerer};
use contract_syntax::{File, ModuleKind};
use contract_types::Ty;

fn is_module_call(file: &File, name: &str) -> bool {
    file.modules.iter().any(|module| {
        module.kind != ModuleKind::Event
            && (!module.options.is_empty() || !module.words.is_empty())
            && (name == format!("{}__options", module.name)
                || module
                    .words
                    .iter()
                    .any(|words| name == format!("{}__{}", module.name, words.platform)))
    })
}

impl Lowerer<'_> {
    pub(crate) fn declare_shapes(&mut self, file: &File) -> Result<(), LowerError> {
        // Authored shapes keep declaration order. Call records are lowered
        // only when a source signature or another shape needs them.
        for shape in &file.shapes {
            if !is_module_call(file, &shape.name) {
                self.ty_id(&Ty::Record(shape.name.clone()))?;
            }
        }
        Ok(())
    }
}
