use std::{
    fmt,
    hash::{Hash, Hasher},
};

use crate::{
    decl_engine::DeclEngine, engine_threading::*, language::ty::*, type_system::*, types::*,
};
use itertools::Itertools;
use sway_ast::Intrinsic;
use sway_error::handler::{ErrorEmitted, Handler};
use sway_types::Span;

#[derive(Debug, Clone)]
pub struct TyIntrinsicFunctionKind {
    pub kind: Intrinsic,
    pub arguments: Vec<TyExpression>,
    pub type_arguments: Vec<TypeArgument>,
    pub span: Span,
}

impl EqWithEngines for TyIntrinsicFunctionKind {}
impl PartialEqWithEngines for TyIntrinsicFunctionKind {
    fn eq(&self, other: &Self, engines: &Engines) -> bool {
        self.kind == other.kind
            && self.arguments.eq(&other.arguments, engines)
            && self.type_arguments.eq(&other.type_arguments, engines)
    }
}

impl HashWithEngines for TyIntrinsicFunctionKind {
    fn hash<H: Hasher>(&self, state: &mut H, engines: &Engines) {
        let TyIntrinsicFunctionKind {
            kind,
            arguments,
            type_arguments,
            // these fields are not hashed because they aren't relevant/a
            // reliable source of obj v. obj distinction
            span: _,
        } = self;
        kind.hash(state);
        arguments.hash(state, engines);
        type_arguments.hash(state, engines);
    }
}

impl SubstTypes for TyIntrinsicFunctionKind {
    fn subst_inner(&mut self, type_mapping: &TypeSubstMap, engines: &Engines) {
        for arg in &mut self.arguments {
            arg.subst(type_mapping, engines);
        }
        for targ in &mut self.type_arguments {
            targ.type_id.subst(type_mapping, engines);
        }
    }
}

impl ReplaceSelfType for TyIntrinsicFunctionKind {
    fn replace_self_type(&mut self, engines: &Engines, self_type: TypeId) {
        for arg in &mut self.arguments {
            arg.replace_self_type(engines, self_type);
        }
        for targ in &mut self.type_arguments {
            targ.type_id.replace_self_type(engines, self_type);
        }
    }
}

impl DebugWithEngines for TyIntrinsicFunctionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, engines: &Engines) -> fmt::Result {
        let targs = self
            .type_arguments
            .iter()
            .map(|targ| format!("{:?}", engines.help_out(targ.type_id)))
            .join(", ");
        let args = self
            .arguments
            .iter()
            .map(|e| format!("{:?}", engines.help_out(e)))
            .join(", ");

        write!(f, "{}::<{}>::({})", self.kind, targs, args)
    }
}

impl DeterministicallyAborts for TyIntrinsicFunctionKind {
    fn deterministically_aborts(&self, decl_engine: &DeclEngine, check_call_body: bool) -> bool {
        matches!(self.kind, Intrinsic::Revert)
            || self
                .arguments
                .iter()
                .any(|x| x.deterministically_aborts(decl_engine, check_call_body))
    }
}

impl CollectTypesMetadata for TyIntrinsicFunctionKind {
    fn collect_types_metadata(
        &self,
        handler: &Handler,
        ctx: &mut CollectTypesMetadataContext,
    ) -> Result<Vec<TypeMetadata>, ErrorEmitted> {
        let mut types_metadata = vec![];
        for type_arg in self.type_arguments.iter() {
            types_metadata.append(&mut type_arg.type_id.collect_types_metadata(handler, ctx)?);
        }
        for arg in self.arguments.iter() {
            types_metadata.append(&mut arg.collect_types_metadata(handler, ctx)?);
        }

        match self.kind {
            Intrinsic::Log => {
                types_metadata.push(TypeMetadata::LoggedType(
                    LogId::new(ctx.log_id_counter()),
                    self.arguments[0].return_type,
                ));
                *ctx.log_id_counter_mut() += 1;
            }
            Intrinsic::Smo => {
                types_metadata.push(TypeMetadata::MessageType(
                    MessageId::new(ctx.message_id_counter()),
                    self.arguments[1].return_type,
                ));
                *ctx.message_id_counter_mut() += 1;
            }
            _ => {}
        }

        Ok(types_metadata)
    }
}
