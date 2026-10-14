//! The structural model of a contract extracted from a parsed file.

use std::marker::PhantomData;

use syn::File;

/// Structural model of a Soroban contract.
///
/// Contract and entrypoint detection populate this model from a parsed
/// [`syn::File`]. The type currently carries only a lifetime placeholder; the
/// detection pass fills it in.
#[derive(Debug, Default)]
pub struct ContractModel<'ast> {
    marker: PhantomData<&'ast ()>,
}

impl<'ast> ContractModel<'ast> {
    /// Builds a model from a parsed file.
    ///
    /// Detection logic is not implemented yet, so this returns an empty model.
    #[must_use]
    pub fn build(_file: &'ast File) -> Self {
        Self {
            marker: PhantomData,
        }
    }
}
