use std::sync::LazyLock;

use super::{Sig, SigChecker, SigSurfaceKind, TyCtx};
use crate::ty::prelude::*;

pub trait ApplyChecker: TyCtx {
    fn apply(&mut self, sig: Sig, arguments: &Interned<ArgsTy>, pol: bool);
}

static EMPTY_ARGS: LazyLock<Interned<ArgsTy>> = LazyLock::new(|| ArgsTy::default().into());

impl Ty {
    /// Call the given type with the given arguments.
    pub fn call(&self, args: &Interned<ArgsTy>, pol: bool, c: &mut impl ApplyChecker) {
        ApplySigChecker(c, args).ty(self, SigSurfaceKind::Call, pol);
    }

    /// Get the tuple element type of the given type.
    pub fn tuple_element_of(&self, pol: bool, c: &mut impl ApplyChecker) {
        ApplySigChecker(c, &EMPTY_ARGS).ty(self, SigSurfaceKind::Array, pol);
    }

    /// Get the element type of the given type.
    pub fn element_of(&self, pol: bool, c: &mut impl ApplyChecker) {
        ApplySigChecker(c, &EMPTY_ARGS).ty(self, SigSurfaceKind::ArrayOrDict, pol);
    }
}

#[derive(BindTyCtx)]
#[bind(0)]
pub struct ApplySigChecker<'a, T: ApplyChecker>(&'a mut T, &'a Interned<ArgsTy>);

impl<T: ApplyChecker> ApplySigChecker<'_, T> {
    fn ty(&mut self, ty: &Ty, surface: SigSurfaceKind, pol: bool) {
        ty.sig_surface(pol, surface, self)
    }
}

impl<T: ApplyChecker> SigChecker for ApplySigChecker<'_, T> {
    fn check(&mut self, cano_sig: Sig, ctx: &mut super::SigCheckContext, pol: bool) -> Option<()> {
        let (cano_sig, is_partialize) = match cano_sig {
            Sig::Partialize(sig) => (*sig, true),
            sig => (sig, false),
        };
        // Bind the arguments to the canonical signature.
        let partial_sig = if ctx.args.is_empty() {
            cano_sig
        } else {
            Sig::With {
                sig: &cano_sig,
                withs: &ctx.args,
                at: &ctx.at,
            }
        };
        let partial_sig = if is_partialize {
            Sig::Partialize(&partial_sig)
        } else {
            partial_sig
        };

        self.0.apply(partial_sig, self.1, pol);
        Some(())
    }
}
