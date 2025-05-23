use typst::foundations::{Dict, Func, Module, Scope, Type};
use typst::syntax::FileId;

use super::BoundChecker;
use crate::{syntax::Decl, ty::prelude::*};

#[derive(Debug, Clone, Copy)]
pub enum Iface<'a> {
    Array(&'a Interned<Ty>),
    Tuple(&'a Interned<Vec<Ty>>),
    Dict(&'a Interned<RecordTy>),
    Content {
        val: &'a typst::foundations::Element,
        at: &'a Ty,
    },
    TypeType {
        val: &'a typst::foundations::Type,
        at: &'a Ty,
    },
    Type {
        val: &'a typst::foundations::Type,
        at: &'a Ty,
    },
    Func {
        val: &'a typst::foundations::Func,
        at: &'a Ty,
    },
    Value {
        val: &'a Dict,
        at: &'a Ty,
    },
    Module {
        val: FileId,
        at: &'a Ty,
    },
    ModuleVal {
        val: &'a Module,
        at: &'a Ty,
    },
}

impl Iface<'_> {
    pub fn to_type(self) -> Ty {
        match self {
            Iface::Array(ty) => Ty::Array(ty.clone()),
            Iface::Tuple(tys) => Ty::Tuple(tys.clone()),
            Iface::Dict(dict) => Ty::Dict(dict.clone()),
            Iface::Content { at, .. }
            | Iface::TypeType { at, .. }
            | Iface::Type { at, .. }
            | Iface::Func { at, .. }
            | Iface::Value { at, .. }
            | Iface::Module { at, .. }
            | Iface::ModuleVal { at, .. } => at.clone(),
        }
    }

    // IfaceShape { iface }
    pub fn select(self, ctx: &mut impl TyCtxMut, key: &StrRef) -> Option<Ty> {
        crate::log_debug_ct!("iface shape: {self:?}");

        match self {
            Iface::Array(..) | Iface::Tuple(..) => {
                select_scope(Some(Type::of::<typst::foundations::Array>().scope()), key)
            }
            Iface::Dict(dict) => dict.field_by_name(key).cloned(),
            Iface::Content { val, .. } => select_scope(Some(val.scope()), key),
            // todo: distinguish TypeType and Type
            Iface::TypeType { val, .. } | Iface::Type { val, .. } => {
                select_scope(Some(val.scope()), key)
            }
            Iface::Func { val, .. } => select_scope(val.scope(), key),
            Iface::Value { val, at: _ } => ctx.type_of_dict(val).field_by_name(key).cloned(),
            Iface::Module { val, at: _ } => ctx.check_module_item(val, key),
            Iface::ModuleVal { val, at: _ } => ctx.type_of_module(val).field_by_name(key).cloned(),
        }
    }
}

fn select_scope(scope: Option<&Scope>, key: &str) -> Option<Ty> {
    let scope = scope?;
    let sub = scope.get(key)?;
    let sub_span = sub.span();
    Some(Ty::Value(InsTy::new_at(sub.read().clone(), sub_span)))
}

pub trait IfaceChecker: TyCtx {
    fn check(&mut self, iface: Iface, ctx: &mut IfaceCheckContext, pol: bool) -> Option<()>;
}

impl Ty {
    /// Iterate over the signatures of the given type.
    pub fn iface_surface(
        &self,
        pol: bool,
        // iface_kind: IfaceSurfaceKind,
        checker: &mut impl IfaceChecker,
    ) {
        let context = IfaceCheckContext { args: Vec::new() };
        let mut worker = IfaceCheckDriver {
            ctx: context,
            checker,
        };

        worker.ty(self, pol);
    }
}

pub struct IfaceCheckContext {
    pub args: Vec<Interned<SigTy>>,
}

#[derive(BindTyCtx)]
#[bind(checker)]
pub struct IfaceCheckDriver<'a> {
    ctx: IfaceCheckContext,
    checker: &'a mut dyn IfaceChecker,
}

impl BoundChecker for IfaceCheckDriver<'_> {
    fn collect(&mut self, ty: &Ty, pol: bool) {
        self.ty(ty, pol);
    }
}

impl IfaceCheckDriver<'_> {
    fn array_as_iface(&self) -> bool {
        true
    }

    fn dict_as_iface(&self) -> bool {
        true
    }

    fn value_as_iface(&self) -> bool {
        true
    }

    fn ty(&mut self, at: &Ty, pol: bool) {
        crate::log_debug_ct!("check iface ty: {at:?}");

        match at {
            Ty::Builtin(BuiltinTy::Stroke) if self.dict_as_iface() => {
                self.checker
                    .check(Iface::Dict(&FLOW_STROKE_DICT), &mut self.ctx, pol);
            }
            Ty::Builtin(BuiltinTy::Margin) if self.dict_as_iface() => {
                self.checker
                    .check(Iface::Dict(&FLOW_MARGIN_DICT), &mut self.ctx, pol);
            }
            Ty::Builtin(BuiltinTy::Inset) if self.dict_as_iface() => {
                self.checker
                    .check(Iface::Dict(&FLOW_INSET_DICT), &mut self.ctx, pol);
            }
            Ty::Builtin(BuiltinTy::Outset) if self.dict_as_iface() => {
                self.checker
                    .check(Iface::Dict(&FLOW_OUTSET_DICT), &mut self.ctx, pol);
            }
            Ty::Builtin(BuiltinTy::Radius) if self.dict_as_iface() => {
                self.checker
                    .check(Iface::Dict(&FLOW_RADIUS_DICT), &mut self.ctx, pol);
            }
            Ty::Builtin(BuiltinTy::TextFont) if self.dict_as_iface() => {
                self.checker
                    .check(Iface::Dict(&FLOW_TEXT_FONT_DICT), &mut self.ctx, pol);
            }
            Ty::Value(ins_ty) => {
                // todo: deduplicate checking early
                if self.value_as_iface() {
                    match &ins_ty.val {
                        Value::Module(val) => {
                            self.checker
                                .check(Iface::ModuleVal { val, at }, &mut self.ctx, pol);
                        }
                        Value::Dict(dict) => {
                            self.checker
                                .check(Iface::Value { val: dict, at }, &mut self.ctx, pol);
                        }
                        Value::Type(ty) => {
                            self.checker
                                .check(Iface::TypeType { val: ty, at }, &mut self.ctx, pol);
                        }
                        Value::Func(func) => {
                            self.checker
                                .check(Iface::Func { val: func, at }, &mut self.ctx, pol);
                        }
                        Value::None
                        | Value::Auto
                        | Value::Bool(_)
                        | Value::Int(_)
                        | Value::Float(_)
                        | Value::Length(..)
                        | Value::Angle(..)
                        | Value::Ratio(..)
                        | Value::Relative(..)
                        | Value::Fraction(..)
                        | Value::Color(..)
                        | Value::Gradient(..)
                        | Value::Tiling(..)
                        | Value::Symbol(..)
                        | Value::Version(..)
                        | Value::Str(..)
                        | Value::Bytes(..)
                        | Value::Label(..)
                        | Value::Datetime(..)
                        | Value::Decimal(..)
                        | Value::Duration(..)
                        | Value::Content(..)
                        | Value::Styles(..)
                        | Value::Array(..)
                        | Value::Args(..)
                        | Value::Dyn(..) => {
                            self.checker.check(
                                Iface::Type {
                                    val: &ins_ty.val.ty(),
                                    at,
                                },
                                &mut self.ctx,
                                pol,
                            );
                        }
                    }
                }
            }
            // todo: more builtin types to check
            Ty::Builtin(BuiltinTy::Content(Some(elem))) if self.value_as_iface() => {
                self.checker
                    .check(Iface::Content { val: elem, at }, &mut self.ctx, pol);
            }
            Ty::Builtin(BuiltinTy::Content(..)) if self.value_as_iface() => {
                let ty = Type::of::<typst::foundations::Content>();
                self.checker
                    .check(Iface::Type { val: &ty, at }, &mut self.ctx, pol);
            }
            Ty::Builtin(BuiltinTy::Type(ty)) if self.value_as_iface() => {
                // todo: distinguish between element and function
                self.checker
                    .check(Iface::Type { val: ty, at }, &mut self.ctx, pol);
            }
            Ty::Builtin(BuiltinTy::Element(elem)) if self.value_as_iface() => {
                self.checker.check(
                    Iface::Func {
                        val: &Func::from(*elem),
                        at,
                    },
                    &mut self.ctx,
                    pol,
                );
            }
            Ty::Builtin(BuiltinTy::Module(module)) => {
                if let Decl::Module(m) = module.as_ref() {
                    self.checker
                        .check(Iface::Module { val: m.fid, at }, &mut self.ctx, pol);
                }
            }
            // Ty::Func(..) if self.value_as_iface() => {
            //     self.checker.check(Iface::Type(sig), &mut self.ctx, pol);
            // }
            // Ty::Array(sig) if self.array_as_sig() => {
            //     // let sig = FlowSignature::array_cons(*sig.clone(), true);
            //     self.checker.check(Iface::ArrayCons(sig), &mut self.ctx, pol);
            // }
            // // todo: tuple
            // Ty::Tuple(_) => {}
            Ty::Dict(sig) if self.dict_as_iface() => {
                // self.check_dict_signature(sig, pol, self.checker);
                self.checker.check(Iface::Dict(sig), &mut self.ctx, pol);
            }
            Ty::Tuple(sig) if self.array_as_iface() => {
                // self.check_dict_signature(sig, pol, self.checker);
                self.checker.check(Iface::Tuple(sig), &mut self.ctx, pol);
            }
            Ty::Array(sig) if self.array_as_iface() => {
                // self.check_dict_signature(sig, pol, self.checker);
                self.checker.check(Iface::Array(sig), &mut self.ctx, pol);
            }
            Ty::Dict(..) => {
                self.checker.check(
                    Iface::Type {
                        val: &Type::of::<typst::foundations::Dict>(),
                        at,
                    },
                    &mut self.ctx,
                    pol,
                );
            }
            Ty::Tuple(..) | Ty::Array(..) => {
                self.checker.check(
                    Iface::Type {
                        val: &Type::of::<typst::foundations::Array>(),
                        at,
                    },
                    &mut self.ctx,
                    pol,
                );
            }
            Ty::Var(..) => at.bounds(pol, self),
            _ if at.has_bounds() => at.bounds(pol, self),
            _ => {}
        }
        // Ty::Select(sel) => sel.ty.bounds(pol, &mut MethodDriver(self,
        // &sel.select)), // todo: calculate these operators
        // Ty::Unary(_) => {}
        // Ty::Binary(_) => {}
        // Ty::If(_) => {}
    }
}
