use super::unsigned::{
    Uint64Type, UintBitwiseLibfunc, UintConstLibfunc, UintDivmodLibfunc, UintEqualLibfunc,
    UintOperationConcreteLibfunc, UintOperationLibfunc, UintSquareRootLibfunc,
    UintToFelt252Libfunc, UintTraits, UintType,
};
use super::IntOperator;
use crate::define_libfunc_hierarchy;
use crate::extensions::bitwise::BitwiseType;
use crate::extensions::felt252::Felt252Type;
use crate::extensions::is_zero::{IsZeroLibfunc, IsZeroTraits};
use crate::extensions::lib_func::{
    BranchSignature, LibfuncSignature, OutputVarInfo, ParamSignature, SierraApChange,
    SignatureSpecializationContext, SpecializationContext,
};
use crate::extensions::range_check::RangeCheckType;
use crate::extensions::{
    GenericLibfunc, NamedType, NoGenericArgsGenericLibfunc, NoGenericArgsGenericType,
    OutputVarReferenceInfo, SpecializationError,
};
use crate::ids::{GenericLibfuncId, GenericTypeId};
use crate::program::GenericArg;

/// Type for u128.
pub type Uint128Type = UintType<Uint128Traits>;

/// A type that contains 4 u128s (a, b, c, d) and guarantees that `a * b = 2**128 * c + d`.
///
/// The guarantee is verified by `u128_mul_guarantee_verify`, which is the only way to destruct this
/// type. This way, one can trust that the guarantee holds although it has not yet been verified.
#[derive(Default)]
pub struct U128MulGuaranteeType;
impl NoGenericArgsGenericType for U128MulGuaranteeType {
    const ID: GenericTypeId = GenericTypeId::new_inline("U128MulGuarantee");
    const STORABLE: bool = true;
    const DUPLICATABLE: bool = false;
    const DROPPABLE: bool = false;
    const ZERO_SIZED: bool = false;
}

define_libfunc_hierarchy! {
    pub enum Uint128Libfunc {
        Operation(UintOperationLibfunc<Uint128Traits>),
        Divmod(UintDivmodLibfunc<Uint128Traits>),
        GuaranteeMul(U128GuaranteeMulLibfunc),
        MulGuaranteeVerify(U128MulGuaranteeVerifyLibfunc),
        Equal(UintEqualLibfunc<Uint128Traits>),
        SquareRoot(UintSquareRootLibfunc<Uint128Traits>),
        Const(UintConstLibfunc<Uint128Traits>),
        FromFelt252(Uint128sFromFelt252Libfunc),
        ToFelt252(UintToFelt252Libfunc<Uint128Traits>),
        IsZero(IsZeroLibfunc<Uint128Traits>),
        Bitwise(UintBitwiseLibfunc<Uint128Traits>),
        ByteReverse(U128ByteReverseLibfunc),
    }, Uint128Concrete
}

#[derive(Default)]
pub struct Uint128Traits;

impl UintTraits for Uint128Traits {
    type UintType = u128;
    const GENERIC_TYPE_ID: GenericTypeId = GenericTypeId::new_inline("u128");
    const IS_SMALL: bool = false;
    const CONST: &'static str = "u128_const";
    const EQUAL: &'static str = "u128_eq";
    const SQUARE_ROOT: &'static str = "u128_sqrt";
    const SQUARE_ROOT_TYPE_ID: GenericTypeId = <Uint64Type as NamedType>::ID;
    const OVERFLOWING_ADD: &'static str = "u128_overflowing_add";
    const OVERFLOWING_SUB: &'static str = "u128_overflowing_sub";
    const TO_FELT252: &'static str = "u128_to_felt252";
    const TRY_FROM_FELT252: &'static str = "u128_try_from_felt252";
    const DIVMOD: &'static str = "u128_safe_divmod";
    const BITWISE: &'static str = "bitwise";
}

impl IsZeroTraits for Uint128Traits {
    const IS_ZERO: &'static str = "u128_is_zero";
    const GENERIC_TYPE_ID: GenericTypeId = <Uint128Type as NamedType>::ID;
}

/// Libfunc for u128 operations.
pub struct Uint128OperationLibfunc {
    pub operator: IntOperator,
}
impl Uint128OperationLibfunc {
    fn new(operator: IntOperator) -> Self {
        Self { operator }
    }
    const OVERFLOWING_ADD: &str = "u128_overflowing_add";
    const OVERFLOWING_SUB: &str = "u128_overflowing_sub";
}
impl GenericLibfunc for Uint128OperationLibfunc {
    type Concrete = UintOperationConcreteLibfunc;

    fn supported_ids() -> Vec<GenericLibfuncId> {
        vec![
            GenericLibfuncId::from(Self::OVERFLOWING_ADD),
            GenericLibfuncId::from(Self::OVERFLOWING_SUB),
        ]
    }

    fn by_id(id: &GenericLibfuncId) -> Option<Self> {
        match id.0.as_str() {
            Self::OVERFLOWING_ADD => Some(Self::new(IntOperator::OverflowingAdd)),
            Self::OVERFLOWING_SUB => Some(Self::new(IntOperator::OverflowingSub)),
            _ => None,
        }
    }

    fn specialize_signature(
        &self,
        context: &dyn SignatureSpecializationContext,
        args: &[GenericArg],
    ) -> Result<LibfuncSignature, SpecializationError> {
        if !args.is_empty() {
            return Err(SpecializationError::WrongNumberOfGenericArgs);
        }
        let range_check_type = context.get_concrete_type(RangeCheckType::id(), &[])?;
        let rc_output_info = OutputVarInfo::new_builtin(range_check_type.clone(), 0);
        let u128_ty = context.get_concrete_type(Uint128Type::id(), &[])?;
        let u128_param = ParamSignature::new(u128_ty.clone());
        let u128_tempvar_output_info =
            OutputVarInfo { ty: u128_ty, ref_info: OutputVarReferenceInfo::NewTempVar { idx: 0 } };
        Ok(LibfuncSignature {
            param_signatures: vec![
                ParamSignature::new(range_check_type).with_allow_add_const(),
                u128_param.clone(),
                u128_param,
            ],
            branch_signatures: vec![
                // No overflow.
                BranchSignature {
                    vars: vec![rc_output_info.clone(), u128_tempvar_output_info.clone()],
                    ap_change: SierraApChange::Known { new_vars_only: false },
                },
                // Overflow.
                BranchSignature {
                    vars: vec![rc_output_info, u128_tempvar_output_info],
                    ap_change: SierraApChange::Known { new_vars_only: false },
                },
            ],
            fallthrough: Some(0),
        })
    }

    fn specialize(
        &self,
        context: &dyn SpecializationContext,
        args: &[GenericArg],
    ) -> Result<Self::Concrete, SpecializationError> {
        Ok(UintOperationConcreteLibfunc {
            operator: self.operator,
            signature: self.specialize_signature(context.upcast(), args)?,
        })
    }
}

/// Libfunc for u128_guarantee_mul.
#[derive(Default)]
pub struct U128GuaranteeMulLibfunc {}
impl NoGenericArgsGenericLibfunc for U128GuaranteeMulLibfunc {
    const STR_ID: &'static str = "u128_guarantee_mul";

    fn specialize_signature(
        &self,
        context: &dyn SignatureSpecializationContext,
    ) -> Result<LibfuncSignature, SpecializationError> {
        let u128_type = context.get_concrete_type(Uint128Type::id(), &[])?;
        Ok(LibfuncSignature::new_non_branch_ex(
            vec![ParamSignature::new(u128_type.clone()), ParamSignature::new(u128_type.clone())],
            vec![
                // High.
                OutputVarInfo {
                    ty: u128_type.clone(),
                    ref_info: OutputVarReferenceInfo::NewTempVar { idx: 0 },
                },
                // Low.
                OutputVarInfo {
                    ty: u128_type,
                    ref_info: OutputVarReferenceInfo::NewTempVar { idx: 1 },
                },
                // Guarantee.
                OutputVarInfo {
                    ty: context.get_concrete_type(U128MulGuaranteeType::id(), &[])?,
                    ref_info: OutputVarReferenceInfo::SimpleDerefs,
                },
            ],
            SierraApChange::Known { new_vars_only: true },
        ))
    }
}

/// Libfunc for u128_mul_guarantee_verify.
#[derive(Default)]
pub struct U128MulGuaranteeVerifyLibfunc {}
impl NoGenericArgsGenericLibfunc for U128MulGuaranteeVerifyLibfunc {
    const STR_ID: &'static str = "u128_mul_guarantee_verify";

    fn specialize_signature(
        &self,
        context: &dyn SignatureSpecializationContext,
    ) -> Result<LibfuncSignature, SpecializationError> {
        let range_check_type = context.get_concrete_type(RangeCheckType::id(), &[])?;
        Ok(LibfuncSignature::new_non_branch_ex(
            vec![
                ParamSignature::new(range_check_type.clone()).with_allow_add_const(),
                ParamSignature::new(context.get_concrete_type(U128MulGuaranteeType::id(), &[])?),
            ],
            vec![OutputVarInfo::new_builtin(range_check_type, 0)],
            SierraApChange::Known { new_vars_only: false },
        ))
    }
}

/// Libfunc for converting a felt252 into a u128, or the number and the overflow in the case of
/// failure.
#[derive(Default)]
pub struct Uint128sFromFelt252Libfunc {}
impl NoGenericArgsGenericLibfunc for Uint128sFromFelt252Libfunc {
    const STR_ID: &'static str = "u128s_from_felt252";

    fn specialize_signature(
        &self,
        context: &dyn SignatureSpecializationContext,
    ) -> Result<LibfuncSignature, SpecializationError> {
        let range_check_type = context.get_concrete_type(RangeCheckType::id(), &[])?;
        let rc_output_info = OutputVarInfo::new_builtin(range_check_type.clone(), 0);
        Ok(LibfuncSignature {
            param_signatures: vec![
                ParamSignature::new(range_check_type).with_allow_add_const(),
                ParamSignature::new(context.get_concrete_type(Felt252Type::id(), &[])?),
            ],
            branch_signatures: vec![
                BranchSignature {
                    vars: vec![
                        rc_output_info.clone(),
                        OutputVarInfo {
                            ty: context.get_concrete_type(Uint128Type::id(), &[])?,
                            ref_info: OutputVarReferenceInfo::SameAsParam { param_idx: 1 },
                        },
                    ],
                    ap_change: SierraApChange::Known { new_vars_only: false },
                },
                BranchSignature {
                    vars: vec![
                        rc_output_info,
                        OutputVarInfo {
                            ty: context.get_concrete_type(Uint128Type::id(), &[])?,
                            ref_info: OutputVarReferenceInfo::NewTempVar { idx: 0 },
                        },
                        OutputVarInfo {
                            ty: context.get_concrete_type(Uint128Type::id(), &[])?,
                            ref_info: OutputVarReferenceInfo::NewTempVar { idx: 1 },
                        },
                    ],
                    ap_change: SierraApChange::Known { new_vars_only: false },
                },
            ],
            fallthrough: Some(0),
        })
    }
}

/// Libfunc for reversing the byte order of a u128.
/// Returns a u128 (and the updated builtin pointer).
#[derive(Default)]
pub struct U128ByteReverseLibfunc {}
impl NoGenericArgsGenericLibfunc for U128ByteReverseLibfunc {
    const STR_ID: &'static str = "u128_byte_reverse";

    fn specialize_signature(
        &self,
        context: &dyn SignatureSpecializationContext,
    ) -> Result<LibfuncSignature, SpecializationError> {
        let bitwise_ty = context.get_concrete_type(BitwiseType::id(), &[])?;
        let u128_ty = context.get_concrete_type(Uint128Type::id(), &[])?;
        Ok(LibfuncSignature::new_non_branch_ex(
            vec![
                ParamSignature::new(bitwise_ty.clone()).with_allow_add_const(),
                ParamSignature::new(u128_ty.clone()),
            ],
            vec![
                // bitwise
                OutputVarInfo::new_builtin(bitwise_ty, 0),
                // result
                OutputVarInfo {
                    ty: u128_ty,
                    ref_info: OutputVarReferenceInfo::NewTempVar { idx: 0 },
                },
            ],
            SierraApChange::Known { new_vars_only: false },
        ))
    }
}
