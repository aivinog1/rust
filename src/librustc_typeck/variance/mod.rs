//! Module for inferring the variance of type and lifetime parameters. See the [rustc dev guide]
//! chapter for more info.
//!
//! [rustc dev guide]: https://rustc-dev-guide.rust-lang.org/variance.html

use hir::Node;
use rustc::ty::query::Providers;
use rustc::ty::{self, CrateVariancesMap, TyCtxt};
use rustc_hir as hir;
use rustc_hir::def_id::{CrateNum, DefId, LOCAL_CRATE};

/// Defines the `TermsContext` basically houses an arena where we can
/// allocate terms.
mod terms;

/// Code to gather up constraints.
mod constraints;

/// Code to solve constraints and write out the results.
mod solve;

/// Code to write unit tests of variance.
pub mod test;

/// Code for transforming variances.
mod xform;

pub fn provide(providers: &mut Providers<'_>) {
    *providers = Providers { variances_of, crate_variances, ..*providers };
}

fn crate_variances(tcx: TyCtxt<'_>, crate_num: CrateNum) -> &CrateVariancesMap<'_> {
    assert_eq!(crate_num, LOCAL_CRATE);
    let mut arena = arena::TypedArena::default();
    let terms_cx = terms::determine_parameters_to_be_inferred(tcx, &mut arena);
    let constraints_cx = constraints::add_constraints_from_crate(terms_cx);
    tcx.arena.alloc(solve::solve_constraints(constraints_cx))
}

fn variances_of(tcx: TyCtxt<'_>, item_def_id: DefId) -> &[ty::Variance] {
    let id = tcx.hir().as_local_hir_id(item_def_id).expect("expected local def-id");
    let unsupported = || {
        // Variance not relevant.
        span_bug!(tcx.hir().span(id), "asked to compute variance for wrong kind of item")
    };
    match tcx.hir().get(id) {
        Node::Item(item) => match item.kind {
            hir::ItemKind::Enum(..)
            | hir::ItemKind::Struct(..)
            | hir::ItemKind::Union(..)
            | hir::ItemKind::Fn(..) => {}

            _ => unsupported(),
        },

        Node::TraitItem(item) => match item.kind {
            hir::TraitItemKind::Fn(..) => {}

            _ => unsupported(),
        },

        Node::ImplItem(item) => match item.kind {
            hir::ImplItemKind::Method(..) => {}

            _ => unsupported(),
        },

        Node::ForeignItem(item) => match item.kind {
            hir::ForeignItemKind::Fn(..) => {}

            _ => unsupported(),
        },

        Node::Variant(_) | Node::Ctor(..) => {}

        _ => unsupported(),
    }

    // Everything else must be inferred.

    let crate_map = tcx.crate_variances(LOCAL_CRATE);
    crate_map.variances.get(&item_def_id).copied().unwrap_or(&[])
}
