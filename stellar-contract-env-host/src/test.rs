use crate::{
    xdr::{
        LedgerEntryData, LedgerKey, LedgerKeyContractData, ScHostFnErrorCode, ScHostObjErrorCode,
        ScObject, ScObjectType, ScStatic, ScStatus, ScVal, ScVec,
    },
    Host, HostError, IntoEnvVal, Object, RawVal, Tag,
};

use crate::xdr;

use im_rc::OrdMap;
use stellar_contract_env_common::{
    xdr::{ScMap, ScMapEntry, WriteXdr},
    CheckedEnv, EnvVal, RawValConvertible,
};

use crate::storage::{AccessType, Footprint, Storage};
use hex::FromHex;

#[cfg(feature = "vm")]
use crate::Vm;
#[cfg(feature = "vm")]
use crate::{
    xdr::{ContractDataEntry, Hash, LedgerEntry, LedgerEntryExt, ScStatusType},
    Symbol,
};
use assert_matches::assert_matches;
#[cfg(feature = "vm")]
use stellar_contract_env_common::Status;

/// numbers test
#[test]
fn u64_roundtrip() -> Result<(), HostError> {
    let host = Host::default();
    let u: u64 = 38473_u64; // This will be treated as a ScVal::Object::U64
    let v = u.into_env_val(&host);
    let obj: Object = v.val.try_into()?;
    assert!(obj.is_obj_type(ScObjectType::U64));
    assert_eq!(obj.get_handle(), 0);
    let j = u64::try_from(v)?;
    assert_eq!(u, j);

    let u2: u64 = u64::MAX; // This will be treated as a ScVal::Object::U64
    let v2 = u2.into_env_val(&host);
    let obj: Object = v2.val.try_into()?;
    assert!(obj.is_obj_type(ScObjectType::U64));
    assert_eq!(obj.get_handle(), 1);
    let k = u64::try_from(v2)?;
    assert_eq!(u2, k);
    Ok(())
}

#[test]
fn i64_roundtrip() -> Result<(), HostError> {
    let host = Host::default();
    let i: i64 = 12345_i64; // Will be treated as ScVal::I64
    let v = i.into_env_val(&host);
    let j = i64::try_from(v)?;
    assert_eq!(i, j);

    let i2: i64 = -13234_i64; // WIll be treated as ScVal::Object::I64
    let v2 = i2.into_env_val(&host);
    let obj: Object = v2.val.try_into()?;
    assert!(obj.is_obj_type(ScObjectType::I64));
    assert_eq!(obj.get_handle(), 0);
    let k = i64::try_from(v2)?;
    assert_eq!(i2, k);
    Ok(())
}

#[test]
fn u32_as_seen_by_host() -> Result<(), HostError> {
    let host = Host::default();
    let scval0 = ScVal::U32(12345);
    let val0 = host.to_host_val(&scval0)?;
    assert!(val0.val.is::<u32>());
    assert!(val0.val.get_tag() == Tag::U32);
    let u = unsafe { <u32 as RawValConvertible>::unchecked_from_val(val0.val) };
    assert_eq!(u, 12345);
    Ok(())
}

#[test]
fn i32_as_seen_by_host() -> Result<(), HostError> {
    let host = Host::default();
    let scval0 = ScVal::I32(-12345);
    let val0 = host.to_host_val(&scval0)?;
    assert!(val0.val.is::<i32>());
    assert!(val0.val.get_tag() == Tag::I32);
    let i = unsafe { <i32 as RawValConvertible>::unchecked_from_val(val0.val) };
    assert_eq!(i, -12345);
    Ok(())
}

#[test]
fn map_put_has_and_get() -> Result<(), HostError> {
    let host = Host::default();
    let scmap: ScMap = vec![
        ScMapEntry {
            key: ScVal::U32(1),
            val: ScVal::U32(2),
        },
        ScMapEntry {
            key: ScVal::U32(2),
            val: ScVal::U32(4),
        },
    ]
    .try_into()?;
    let scobj = ScObject::Map(scmap);
    let obj = host.to_host_obj(&scobj)?;
    let k: RawVal = 3_u32.into();
    let v: RawVal = 6_u32.into();
    assert!(!bool::try_from(host.map_has(obj.to_object(), k)?)?);
    let obj1 = host.map_put(obj.to_object(), k, v)?;
    assert!(bool::try_from(host.map_has(obj1, k)?)?);
    let rv = host.map_get(obj1, k)?;
    let v = unsafe { <u32 as RawValConvertible>::unchecked_from_val(rv) };
    assert_eq!(v, 6);
    Ok(())
}

/// Vec test
#[test]
fn vec_as_seen_by_host() -> Result<(), HostError> {
    let host = Host::default();
    let scvec0: ScVec = ScVec(vec![ScVal::U32(1)].try_into()?);
    let scvec1: ScVec = ScVec(vec![ScVal::U32(1)].try_into()?);
    let scobj0: ScObject = ScObject::Vec(scvec0);
    let scobj1: ScObject = ScObject::Vec(scvec1);
    let scval0 = ScVal::Object(Some(scobj0));
    let scval1 = ScVal::Object(Some(scobj1));
    let val0 = host.to_host_val(&scval0)?;
    let val1 = host.to_host_val(&scval1)?;
    assert!(val0.val.is::<Object>());
    assert!(val1.val.is::<Object>());
    let obj0: Object = val0.val.try_into()?;
    let obj1: Object = val1.val.try_into()?;
    assert_eq!(obj0.get_handle(), 0);
    assert_eq!(obj1.get_handle(), 1);
    assert!(obj0.is_obj_type(ScObjectType::Vec));
    assert!(obj1.is_obj_type(ScObjectType::Vec));
    // Check that we got 2 distinct Vec objects
    assert_ne!(val0.val.get_payload(), val1.val.get_payload());
    // But also that they compare deep-equal.
    assert_eq!(val0, val1);
    Ok(())
}

#[test]
fn vec_front_and_back() -> Result<(), HostError> {
    let host = Host::default();
    let scvec: ScVec = vec![ScVal::U32(1), ScVal::U32(2), ScVal::U32(3)].try_into()?;
    let scobj = ScObject::Vec(scvec);
    let obj = host.to_host_obj(&scobj)?;
    let front =
        unsafe { <i32 as RawValConvertible>::unchecked_from_val(host.vec_front(obj.to_object())?) };
    let back =
        unsafe { <i32 as RawValConvertible>::unchecked_from_val(host.vec_back(obj.to_object())?) };
    assert_eq!(front, 1);
    assert_eq!(back, 3);
    Ok(())
}

#[test]
fn empty_vec_front() -> Result<(), HostError> {
    let host = Host::default();
    let scvec: ScVec = vec![].try_into()?;
    let scobj = ScObject::Vec(scvec);
    let obj = host.to_host_obj(&scobj)?;
    assert_matches!(
        host.vec_front(obj.to_object()),
        Err(HostError::WithStatus(
            _,
            ScStatus::HostObjectError(ScHostObjErrorCode::VecIndexOutOfBound)
        ))
    );
    Ok(())
}

#[test]
fn empty_vec_back() -> Result<(), HostError> {
    let host = Host::default();
    let scvec: ScVec = vec![].try_into()?;
    let scobj = ScObject::Vec(scvec);
    let obj = host.to_host_obj(&scobj)?;
    assert_matches!(
        host.vec_back(obj.to_object()),
        Err(HostError::WithStatus(
            _,
            ScStatus::HostObjectError(ScHostObjErrorCode::VecIndexOutOfBound)
        ))
    );
    Ok(())
}

#[test]
fn vec_put_and_get() -> Result<(), HostError> {
    let host = Host::default();
    let scvec: ScVec = vec![ScVal::U32(1), ScVal::U32(2), ScVal::U32(3)].try_into()?;
    let scobj = ScObject::Vec(scvec);
    let obj = host.to_host_obj(&scobj)?;
    let i: RawVal = 1_u32.into();
    let obj1 = host.vec_put(obj.to_object(), i, 9_u32.into())?;
    let rv = host.vec_get(obj1, i)?;
    let v = unsafe { <u32 as RawValConvertible>::unchecked_from_val(rv) };
    assert_eq!(v, 9);
    Ok(())
}

#[test]
fn vec_push_pop_and_len() -> Result<(), HostError> {
    let host = Host::default();
    let scvec: ScVec = vec![].try_into()?;
    let scobj = ScObject::Vec(scvec);
    let obj = host.to_host_obj(&scobj)?;
    let l =
        unsafe { <u32 as RawValConvertible>::unchecked_from_val(host.vec_len(obj.to_object())?) };
    assert_eq!(l, 0);
    let obj1 = host.vec_push(obj.to_object(), 1u32.into())?;
    let obj2 = host.vec_push(obj1, 2u32.into())?;
    let l = unsafe { <u32 as RawValConvertible>::unchecked_from_val(host.vec_len(obj2)?) };
    assert_eq!(l, 2);
    let obj3 = host.vec_pop(obj2)?;
    let l = unsafe { <u32 as RawValConvertible>::unchecked_from_val(host.vec_len(obj3)?) };
    assert_eq!(l, 1);
    let obj4 = host.vec_pop(obj3)?;
    let l = unsafe { <u32 as RawValConvertible>::unchecked_from_val(host.vec_len(obj4)?) };
    assert_eq!(l, 0);
    Ok(())
}

#[test]
fn vec_pop_empty_vec() -> Result<(), HostError> {
    let host = Host::default();
    let scvec: ScVec = vec![].try_into()?;
    let scobj = ScObject::Vec(scvec);
    let obj = host.to_host_obj(&scobj)?;
    assert_matches!(
        host.vec_pop(obj.to_object()),
        Err(HostError::WithStatus(
            _,
            ScStatus::HostObjectError(ScHostObjErrorCode::VecIndexOutOfBound)
        ))
    );
    Ok(())
}

#[test]
fn vec_get_out_of_bound() -> Result<(), HostError> {
    let host = Host::default();
    let scvec: ScVec = vec![ScVal::U32(1), ScVal::U32(2), ScVal::U32(3)].try_into()?;
    let scobj = ScObject::Vec(scvec);
    let obj = host.to_host_obj(&scobj)?;
    assert_matches!(
        host.vec_get(obj.to_object(), 3_u32.into()),
        Err(HostError::WithStatus(
            _,
            ScStatus::HostObjectError(ScHostObjErrorCode::VecIndexOutOfBound)
        ))
    );
    Ok(())
}

#[test]
fn vec_get_wrong_index_type() -> Result<(), HostError> {
    let host = Host::default();
    let scvec: ScVec = vec![].try_into()?;
    let scobj = ScObject::Vec(scvec);
    let obj = host.to_host_obj(&scobj)?;
    assert_matches!(
        host.vec_get(obj.to_object(), (-1_i32).into()),
        Err(HostError::WithStatus(
            _,
            ScStatus::HostFunctionError(ScHostFnErrorCode::InputArgsWrongType)
        ))
    );
    Ok(())
}

#[test]
fn vec_del_and_cmp() -> Result<(), HostError> {
    let host = Host::default();
    let scvec: ScVec = vec![ScVal::U32(1), ScVal::U32(2), ScVal::U32(3)].try_into()?;
    let obj = host.to_host_obj(&ScObject::Vec(scvec))?;
    let obj1 = host.vec_del(obj.to_object(), 1u32.into())?;
    let scvec_ref: ScVec = vec![ScVal::U32(1), ScVal::U32(3)].try_into()?;
    let obj_ref = host.to_host_obj(&ScObject::Vec(scvec_ref))?;
    assert_eq!(host.obj_cmp(obj1.into(), obj_ref.into())?, 0);
    Ok(())
}

#[test]
fn vec_del_out_of_bound() -> Result<(), HostError> {
    let host = Host::default();
    let scvec: ScVec = vec![ScVal::U32(1), ScVal::U32(2), ScVal::U32(3)].try_into()?;
    let scobj = ScObject::Vec(scvec);
    let obj = host.to_host_obj(&scobj)?;
    assert_matches!(
        host.vec_del(obj.to_object(), 3_u32.into()),
        Err(HostError::WithStatus(
            _,
            ScStatus::HostObjectError(ScHostObjErrorCode::VecIndexOutOfBound)
        ))
    );
    Ok(())
}

#[test]
fn vec_del_wrong_index_type() -> Result<(), HostError> {
    let host = Host::default();
    let scvec: ScVec = vec![].try_into()?;
    let scobj = ScObject::Vec(scvec);
    let obj = host.to_host_obj(&scobj)?;
    assert_matches!(
        host.vec_del(obj.to_object(), (-1_i32).into()),
        Err(HostError::WithStatus(
            _,
            ScStatus::HostFunctionError(ScHostFnErrorCode::InputArgsWrongType)
        ))
    );
    Ok(())
}

#[test]
fn vec_slice_and_cmp() -> Result<(), HostError> {
    let host = Host::default();
    let scvec: ScVec = vec![ScVal::U32(1), ScVal::U32(2), ScVal::U32(3)].try_into()?;
    let obj = host.to_host_obj(&ScObject::Vec(scvec))?;
    let obj1 = host.vec_slice(obj.to_object(), 1u32.into(), 3u32.into())?;
    let scvec_ref: ScVec = vec![ScVal::U32(2), ScVal::U32(3)].try_into()?;
    let obj_ref = host.to_host_obj(&ScObject::Vec(scvec_ref))?;
    assert_eq!(host.obj_cmp(obj1.into(), obj_ref.into())?, 0);

    let obj2 = host.vec_slice(obj.to_object(), 0u32.into(), 3u32.into())?;
    assert_ne!(obj2.as_ref().get_payload(), obj.as_raw().get_payload());
    assert_eq!(host.obj_cmp(obj2.into(), obj.into())?, 0);
    Ok(())
}

#[test]
fn vec_slice_start_equal_to_end() -> Result<(), HostError> {
    let host = Host::default();
    let vec = ScObject::Vec(vec![ScVal::U32(1), ScVal::U32(2), ScVal::U32(3)].try_into()?);
    let slice = host.from_host_obj(host.vec_slice(
        host.to_host_obj(&vec)?.to_object(),
        1_u32.into(),
        1_u32.into(),
    )?)?;
    let want = ScObject::Vec(vec![].try_into()?);
    assert_eq!(slice, want);
    Ok(())
}

#[test]
fn vec_slice_start_greater_than_end() -> Result<(), HostError> {
    let host = Host::default();
    let vec = ScObject::Vec(vec![ScVal::U32(1), ScVal::U32(2), ScVal::U32(3)].try_into()?);
    let slice_result = host.vec_slice(
        host.to_host_obj(&vec)?.to_object(),
        2_u32.into(),
        1_u32.into(),
    );
    assert_matches!(
        slice_result,
        Err(HostError::WithStatus(
            _,
            ScStatus::HostFunctionError(ScHostFnErrorCode::InputArgsInvalid)
        ))
    );
    Ok(())
}

#[test]
fn vec_slice_start_out_of_bound() -> Result<(), HostError> {
    let host = Host::default();
    let scvec: ScVec = vec![ScVal::U32(1), ScVal::U32(2), ScVal::U32(3)].try_into()?;
    let scobj = ScObject::Vec(scvec);
    let obj = host.to_host_obj(&scobj)?;
    assert_matches!(
        host.vec_slice(obj.to_object(), 0_u32.into(), 4_u32.into()),
        Err(HostError::WithStatus(
            _,
            ScStatus::HostObjectError(ScHostObjErrorCode::VecIndexOutOfBound)
        ))
    );
    Ok(())
}

#[test]
fn vec_slice_end_out_of_bound() -> Result<(), HostError> {
    let host = Host::default();
    let scvec: ScVec = vec![ScVal::U32(1), ScVal::U32(2), ScVal::U32(3)].try_into()?;
    let scobj = ScObject::Vec(scvec);
    let obj = host.to_host_obj(&scobj)?;
    assert_matches!(
        host.vec_slice(obj.to_object(), 0_u32.into(), 4_u32.into()),
        Err(HostError::WithStatus(
            _,
            ScStatus::HostObjectError(ScHostObjErrorCode::VecIndexOutOfBound)
        ))
    );
    Ok(())
}

#[test]
fn vec_take_wrong_index_type() -> Result<(), HostError> {
    let host = Host::default();
    let scvec: ScVec = vec![].try_into()?;
    let scobj = ScObject::Vec(scvec);
    let obj = host.to_host_obj(&scobj)?;
    assert_matches!(
        host.vec_slice(obj.to_object(), (-1_i32).into(), 1_u32.into()),
        Err(HostError::WithStatus(
            _,
            ScStatus::HostFunctionError(ScHostFnErrorCode::InputArgsWrongType)
        ))
    );
    Ok(())
}

#[test]
fn vec_take_wrong_len_type() -> Result<(), HostError> {
    let host = Host::default();
    let scvec: ScVec = vec![].try_into()?;
    let scobj = ScObject::Vec(scvec);
    let obj = host.to_host_obj(&scobj)?;
    assert_matches!(
        host.vec_slice(obj.to_object(), 1_u32.into(), (-1_i32).into()),
        Err(HostError::WithStatus(
            _,
            ScStatus::HostFunctionError(ScHostFnErrorCode::InputArgsWrongType)
        ))
    );
    Ok(())
}

#[test]
fn vec_insert_and_cmp() -> Result<(), HostError> {
    let host = Host::default();
    let scvec: ScVec = vec![ScVal::U32(2)].try_into()?;
    let obj = host.to_host_obj(&ScObject::Vec(scvec))?;
    let obj1 = host.vec_insert(obj.to_object(), 0u32.into(), 1u32.into())?;
    let scvec_ref: ScVec = vec![ScVal::U32(1), ScVal::U32(2)].try_into()?;
    let obj_ref = host.to_host_obj(&ScObject::Vec(scvec_ref))?;
    assert_eq!(host.obj_cmp(obj1.into(), obj_ref.into())?, 0);

    let obj2 = host.vec_insert(obj1, 2u32.into(), 3u32.into())?;
    let scvec_ref: ScVec = vec![ScVal::U32(1), ScVal::U32(2), ScVal::U32(3)].try_into()?;
    let obj_ref = host.to_host_obj(&ScObject::Vec(scvec_ref))?;
    assert_eq!(host.obj_cmp(obj2.into(), obj_ref.into())?, 0);
    Ok(())
}

#[test]
fn vec_insert_out_of_bound() -> Result<(), HostError> {
    let host = Host::default();
    let scvec: ScVec = vec![ScVal::U32(1), ScVal::U32(2), ScVal::U32(3)].try_into()?;
    let scobj = ScObject::Vec(scvec);
    let obj = host.to_host_obj(&scobj)?;
    assert_matches!(
        host.vec_insert(obj.to_object(), 4_u32.into(), 9u32.into()),
        Err(HostError::WithStatus(
            _,
            ScStatus::HostObjectError(ScHostObjErrorCode::VecIndexOutOfBound)
        ))
    );
    Ok(())
}

#[test]
fn vec_insert_wrong_index_type() -> Result<(), HostError> {
    let host = Host::default();
    let scvec: ScVec = vec![].try_into()?;
    let scobj = ScObject::Vec(scvec);
    let obj = host.to_host_obj(&scobj)?;
    assert_matches!(
        host.vec_insert(obj.to_object(), (-1_i32).into(), 9u32.into()),
        Err(HostError::WithStatus(
            _,
            ScStatus::HostFunctionError(ScHostFnErrorCode::InputArgsWrongType)
        ))
    );
    Ok(())
}

#[test]
fn vec_append() -> Result<(), HostError> {
    let host = Host::default();
    let scvec0: ScVec = vec![ScVal::U32(1), ScVal::U32(2), ScVal::U32(3)].try_into()?;
    let obj0 = host.to_host_obj(&ScObject::Vec(scvec0))?;
    let scvec1: ScVec = vec![ScVal::U32(4), ScVal::U32(5), ScVal::U32(6)].try_into()?;
    let obj1 = host.to_host_obj(&ScObject::Vec(scvec1))?;
    let obj2 = host.vec_append(*obj0.as_ref(), *obj1.as_ref())?;
    let scvec_ref: ScVec = vec![
        ScVal::U32(1),
        ScVal::U32(2),
        ScVal::U32(3),
        ScVal::U32(4),
        ScVal::U32(5),
        ScVal::U32(6),
    ]
    .try_into()?;
    let obj_ref = host.to_host_obj(&ScObject::Vec(scvec_ref))?;
    assert_eq!(host.obj_cmp(obj2.into(), obj_ref.into())?, 0);
    Ok(())
}

#[test]
fn vec_append_empty() -> Result<(), HostError> {
    let host = Host::default();
    let scvec0: ScVec = vec![].try_into()?;
    let obj0 = host.to_host_obj(&ScObject::Vec(scvec0))?;
    let obj1 = host.vec_append(*obj0.as_ref(), *obj0.as_ref())?;
    assert_ne!(obj0.as_raw().get_payload(), obj1.as_ref().get_payload());
    assert_eq!(host.obj_cmp(obj0.into(), obj1.into())?, 0);
    Ok(())
}

/// crypto tests
#[test]
fn sha256_test() -> Result<(), HostError> {
    let host = Host::default();
    let obj0 = host.to_host_obj(&ScObject::Binary(vec![1].try_into()?))?;
    let hash_obj = host.compute_hash_sha256(obj0.to_object())?;

    let v = host.from_host_val(hash_obj.to_raw())?;
    let bin = match v {
        ScVal::Object(Some(scobj)) => match scobj {
            ScObject::Binary(bin) => bin,
            _ => panic!("Wrong type"),
        },
        _ => panic!("Wrong type"),
    };

    /*
    We took the sha256 of [1], which is 4bf5122f344554c53bde2ebb8cd2b7e3d1600ad631c385a5d7cce23c7785459a
    The exp array contains the decimal representation of each hex value
    */
    let exp: Vec<u8> = vec![
        75, 245, 18, 47, 52, 69, 84, 197, 59, 222, 46, 187, 140, 210, 183, 227, 209, 96, 10, 214,
        49, 195, 133, 165, 215, 204, 226, 60, 119, 133, 69, 154,
    ];
    assert_eq!(bin.as_vec().clone(), exp);
    Ok(())
}

#[test]
fn ed25519_verify_test() -> Result<(), HostError> {
    let host = Host::default();

    // From https://datatracker.ietf.org/doc/html/rfc8032#section-7.1

    // First verify successfully
    let public_key: &[u8] = b"3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c";
    let message: &[u8] = b"72";
    let signature: &[u8] = b"92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00";

    let pub_bytes: Vec<u8> = FromHex::from_hex(public_key).unwrap();
    let msg_bytes: Vec<u8> = FromHex::from_hex(message).unwrap();
    let sig_bytes: Vec<u8> = FromHex::from_hex(signature).unwrap();

    let obj_pub = host.to_host_obj(&ScObject::Binary(pub_bytes.try_into()?))?;
    let obj_msg = host.to_host_obj(&ScObject::Binary(msg_bytes.try_into()?))?;
    let obj_sig = host.to_host_obj(&ScObject::Binary(sig_bytes.try_into()?))?;

    let res = host.verify_sig_ed25519(
        obj_msg.to_object(),
        obj_pub.to_object(),
        obj_sig.to_object(),
    );

    res.expect("verification failed");

    // Now verify with wrong message
    let message2: &[u8] = b"73";
    let msg_bytes2: Vec<u8> = FromHex::from_hex(message2).unwrap();
    let obj_msg2 = host.to_host_obj(&ScObject::Binary(msg_bytes2.try_into()?))?;

    let res_failed = host.verify_sig_ed25519(
        obj_msg2.to_object(),
        obj_pub.to_object(),
        obj_sig.to_object(),
    );

    match res_failed {
        Ok(_) => panic!("verification test failed"),
        _ => (),
    };
    Ok(())
}

fn sha256_hash_id_preimage(pre_image: xdr::HashIdPreimage) -> xdr::Hash {
    use sha2::{Digest, Sha256};

    let mut buf = Vec::new();
    pre_image
        .write_xdr(&mut buf)
        .expect("preimage write failed");

    xdr::Hash(Sha256::digest(buf).try_into().expect("invalid hash"))
}

fn check_new_code(host: &Host, storage_key: LedgerKey, code: ScVal) {
    host.visit_storage(|s: &mut Storage| {
        assert!(s.has(&storage_key)?);

        match s.get(&storage_key)?.data {
            LedgerEntryData::ContractData(cde) => assert_eq!(cde.val, code),
            _ => panic!("expected contract data"),
        };
        Ok(())
    })
    .unwrap();
}

/// create contract tests
fn create_contract_test_helper(
    secret: &[u8],
    public: &[u8],
    salt: &[u8],
    code: &[u8],
    sig_preimage: Vec<u8>,
) -> Result<Host, HostError> {
    use ed25519_dalek::{
        Keypair, PublicKey, SecretKey, Signature, Signer, PUBLIC_KEY_LENGTH, SECRET_KEY_LENGTH,
    };
    use sha2::{Digest, Sha256};

    let sec_bytes: Vec<u8> = FromHex::from_hex(secret).unwrap();
    let pub_bytes: Vec<u8> = FromHex::from_hex(public).unwrap();
    let salt_bytes: Vec<u8> = FromHex::from_hex(salt).unwrap();

    let secret: SecretKey = SecretKey::from_bytes(&sec_bytes[..SECRET_KEY_LENGTH]).unwrap();
    let public: PublicKey = PublicKey::from_bytes(&pub_bytes[..PUBLIC_KEY_LENGTH]).unwrap();
    let keypair: Keypair = Keypair {
        secret: secret,
        public: public,
    };

    // Create signature
    let signature: Signature = keypair.sign(Sha256::digest(sig_preimage).as_slice());

    // Make contractID so we can include it in the footprint
    let pre_image = xdr::HashIdPreimage::ContractIdFromEd25519(xdr::HashIdPreimageContractId {
        ed25519: xdr::Uint256(pub_bytes.as_slice().try_into().unwrap()),
        salt: xdr::Uint256(salt_bytes.as_slice().try_into().unwrap()),
    });

    let hash = sha256_hash_id_preimage(pre_image);

    let hash_copy = hash.clone();
    let key = ScVal::Static(ScStatic::LedgerKeyContractCodeWasm);
    let storage_key = LedgerKey::ContractData(LedgerKeyContractData {
        contract_id: hash,
        key,
    });

    let mut footprint = Footprint::default();
    footprint.record_access(&storage_key, AccessType::ReadWrite);

    // Initialize storage and host
    let storage = Storage::with_enforcing_footprint_and_map(footprint, OrdMap::new());
    let host = Host::with_storage(storage);

    // Create contract
    let obj_code = host.to_host_obj(&ScObject::Binary(code.try_into()?))?;
    let obj_pub = host.to_host_obj(&ScObject::Binary(pub_bytes.try_into()?))?;
    let obj_salt = host.to_host_obj(&ScObject::Binary(salt_bytes.try_into()?))?;
    let obj_sig = host.to_host_obj(&ScObject::Binary(signature.to_bytes().try_into()?))?;

    let contract_id = host.create_contract(
        obj_code.to_object(),
        obj_salt.to_object(),
        obj_pub.to_object(),
        obj_sig.to_object(),
    )?;

    let v = host.from_host_val(contract_id.to_raw())?;
    let bin = match v {
        ScVal::Object(Some(scobj)) => match scobj {
            ScObject::Binary(bin) => bin,
            _ => panic!("Wrong type"),
        },
        _ => panic!("Wrong type"),
    };

    if bin.as_slice() != hash_copy.0.as_slice() {
        panic!("return value doesn't match")
    }

    check_new_code(
        &host,
        storage_key,
        ScVal::Object(Some(ScObject::Binary(code.try_into()?))),
    );

    Ok(host)
}

#[test]
fn create_contract_test() -> Result<(), HostError> {
    let secret_key: &[u8] = b"9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
    let public_key: &[u8] = b"d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
    let salt: &[u8] = b"3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c";
    let code: &[u8] = b"70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4";

    let separator =
        "create_contract(nonce: u256, contract: Vec<u8>, salt: u256, key: u256, sig: Vec<u8>)";

    let salt_bytes: Vec<u8> = FromHex::from_hex(salt).unwrap();
    let sig_preimage = [separator.as_bytes(), salt_bytes.as_slice(), code].concat();

    // valid signature
    assert!(
        create_contract_test_helper(secret_key, public_key, salt, code, sig_preimage.clone())
            .is_ok()
    );

    //Incorrect separator
    let bad_sep = ["d".as_bytes(), salt_bytes.as_slice(), code].concat();
    assert!(create_contract_test_helper(secret_key, public_key, salt, code, bad_sep).is_err());

    // wrong public key
    let bad_pub: &[u8] = b"3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c";
    assert!(create_contract_test_helper(secret_key, bad_pub, salt, code, sig_preimage).is_err());
    Ok(())
}

/// VM tests

/* Example: rs-stellar-contract-sdk/examples/create_contract/src/lib.rs
This is an example WASM from the SDK that creates another contract using the
create_contract_using_parent_id host function.

To regenerate:
- Nightly toolchain is a prerequisite - 'rustup component add rust-src --toolchain nightly'
- `make build` in rs-stellar-contract-sdk
- `wasm-strip rs-stellar-contract-sdk/target-tiny/wasm32-unknown-unknown/release/example_create_contract.wasm`
- `xxd -i rs-stellar-contract-sdk/target-tiny/wasm32-unknown-unknown/release/example_create_contract.wasm`
*/
#[cfg(feature = "vm")]
#[test]
fn create_contract_using_parent_id_test() {
    //This contract creates another contract
    let code: [u8; 269] = [
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x0c, 0x02, 0x60, 0x02, 0x7e, 0x7e,
        0x01, 0x7e, 0x60, 0x02, 0x7f, 0x7e, 0x00, 0x02, 0x07, 0x01, 0x01, 0x6c, 0x01, 0x34, 0x00,
        0x00, 0x03, 0x03, 0x02, 0x00, 0x01, 0x05, 0x03, 0x01, 0x00, 0x10, 0x06, 0x19, 0x03, 0x7f,
        0x01, 0x41, 0x80, 0x80, 0xc0, 0x00, 0x0b, 0x7f, 0x00, 0x41, 0x80, 0x80, 0xc0, 0x00, 0x0b,
        0x7f, 0x00, 0x41, 0x80, 0x80, 0xc0, 0x00, 0x0b, 0x07, 0x2e, 0x04, 0x06, 0x6d, 0x65, 0x6d,
        0x6f, 0x72, 0x79, 0x02, 0x00, 0x06, 0x63, 0x72, 0x65, 0x61, 0x74, 0x65, 0x00, 0x01, 0x0a,
        0x5f, 0x5f, 0x64, 0x61, 0x74, 0x61, 0x5f, 0x65, 0x6e, 0x64, 0x03, 0x01, 0x0b, 0x5f, 0x5f,
        0x68, 0x65, 0x61, 0x70, 0x5f, 0x62, 0x61, 0x73, 0x65, 0x03, 0x02, 0x0a, 0x96, 0x01, 0x02,
        0x7d, 0x01, 0x02, 0x7f, 0x23, 0x00, 0x41, 0x30, 0x6b, 0x22, 0x02, 0x24, 0x00, 0x20, 0x02,
        0x41, 0x18, 0x6a, 0x20, 0x00, 0x10, 0x02, 0x02, 0x40, 0x20, 0x02, 0x29, 0x03, 0x18, 0xa7,
        0x0d, 0x00, 0x20, 0x02, 0x29, 0x03, 0x20, 0x21, 0x00, 0x20, 0x02, 0x41, 0x08, 0x6a, 0x20,
        0x01, 0x10, 0x02, 0x20, 0x02, 0x29, 0x03, 0x08, 0xa7, 0x0d, 0x00, 0x20, 0x02, 0x29, 0x03,
        0x10, 0x21, 0x01, 0x23, 0x00, 0x41, 0x10, 0x6b, 0x22, 0x03, 0x24, 0x00, 0x02, 0x40, 0x20,
        0x00, 0x42, 0x0f, 0x83, 0x42, 0x07, 0x52, 0x20, 0x01, 0x42, 0x0f, 0x83, 0x42, 0x07, 0x52,
        0x72, 0x45, 0x04, 0x40, 0x20, 0x00, 0x20, 0x01, 0x10, 0x00, 0x1a, 0x0c, 0x01, 0x0b, 0x00,
        0x0b, 0x20, 0x03, 0x41, 0x10, 0x6a, 0x24, 0x00, 0x20, 0x02, 0x41, 0x30, 0x6a, 0x24, 0x00,
        0x42, 0x05, 0x0f, 0x0b, 0x00, 0x0b, 0x16, 0x00, 0x20, 0x00, 0x20, 0x01, 0x37, 0x03, 0x08,
        0x20, 0x00, 0x20, 0x01, 0xa7, 0x41, 0xc7, 0x00, 0x47, 0xad, 0x37, 0x03, 0x00, 0x0b,
    ];

    let secret_key: &[u8] = b"9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
    let public_key: &[u8] = b"d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
    let salt: &[u8] = b"3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c";

    let separator =
        "create_contract(nonce: u256, contract: Vec<u8>, salt: u256, key: u256, sig: Vec<u8>)";

    let salt_bytes: Vec<u8> = FromHex::from_hex(salt).unwrap();
    let sig_preimage = [separator.as_bytes(), salt_bytes.as_slice(), code.as_slice()].concat();

    let host = create_contract_test_helper(
        secret_key,
        public_key,
        salt,
        code.as_slice(),
        sig_preimage.clone(),
    )
    .unwrap();

    // Get parent contractID
    let pub_bytes: Vec<u8> = FromHex::from_hex(public_key).unwrap();
    let pre_image = xdr::HashIdPreimage::ContractIdFromEd25519(xdr::HashIdPreimageContractId {
        ed25519: xdr::Uint256(pub_bytes.as_slice().try_into().unwrap()),
        salt: xdr::Uint256(salt_bytes.as_slice().try_into().unwrap()),
    });

    let parent_id = sha256_hash_id_preimage(pre_image);

    //Put child contract that will be created into the footprint
    //Use the same salt
    let child_pre_image =
        xdr::HashIdPreimage::ContractIdFromContract(xdr::HashIdPreimageChildContractId {
            contract_id: parent_id.clone(),
            salt: xdr::Uint256(salt_bytes.as_slice().try_into().unwrap()),
        });

    let child_id = sha256_hash_id_preimage(child_pre_image);

    let child_storage_key = LedgerKey::ContractData(LedgerKeyContractData {
        contract_id: child_id,
        key: ScVal::Static(ScStatic::LedgerKeyContractCodeWasm),
    });

    host.visit_storage(|s: &mut Storage| {
        s.footprint
            .record_access(&child_storage_key, AccessType::ReadWrite);
        Ok(())
    })
    .unwrap();

    // prepare arguments
    let child_code: &[u8] = b"70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4";
    let code_val = ScVal::Object(Some(ScObject::Binary(child_code.try_into().unwrap())));

    let sym = Symbol::from_str("create");
    let salt_bytes: Vec<u8> = FromHex::from_hex(salt).unwrap();
    let scvec0: ScVec = vec![
        code_val.clone(),
        ScVal::Object(Some(ScObject::Binary(salt_bytes.try_into().unwrap()))),
    ]
    .try_into()
    .unwrap();
    let args = host.to_host_obj(&ScObject::Vec(scvec0)).unwrap();

    let p_id_sobj = ScObject::Binary(parent_id.0.try_into().unwrap());
    let p_id_obj = host.to_host_obj(&p_id_sobj).unwrap();
    host.call(p_id_obj.to_object(), sym.into(), args.into())
        .unwrap();

    //Validate child contract exists and code is what we expected
    check_new_code(&host, child_storage_key, code_val);
}

/* Example: rs-stellar-contract-sdk/examples/add_i32/src/lib.rs
This is an example WASM from the SDK that unpacks two SCV_I32 arguments, adds
them with an overflow check, and re-packs them as an SCV_I32 if successful.

To regenerate:
- Nightly toolchain is a prerequisite - 'rustup component add rust-src --toolchain nightly'
- `make build` in rs-stellar-contract-sdk
- `wasm-strip rs-stellar-contract-sdk/target-tiny/wasm32-unknown-unknown/release/example_add_i32.wasm`
- `xxd -i rs-stellar-contract-sdk/target-tiny/wasm32-unknown-unknown/release/example_add_i32.wasm`
*/

#[cfg(feature = "vm")]
#[test]
fn invoke_single_contract_function() -> Result<(), HostError> {
    let host = Host::default();
    let code: [u8; 163] = [
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x07, 0x01, 0x60, 0x02, 0x7e, 0x7e,
        0x01, 0x7e, 0x03, 0x02, 0x01, 0x00, 0x05, 0x03, 0x01, 0x00, 0x10, 0x06, 0x11, 0x02, 0x7f,
        0x00, 0x41, 0x80, 0x80, 0xc0, 0x00, 0x0b, 0x7f, 0x00, 0x41, 0x80, 0x80, 0xc0, 0x00, 0x0b,
        0x07, 0x2b, 0x04, 0x06, 0x6d, 0x65, 0x6d, 0x6f, 0x72, 0x79, 0x02, 0x00, 0x03, 0x61, 0x64,
        0x64, 0x00, 0x00, 0x0a, 0x5f, 0x5f, 0x64, 0x61, 0x74, 0x61, 0x5f, 0x65, 0x6e, 0x64, 0x03,
        0x00, 0x0b, 0x5f, 0x5f, 0x68, 0x65, 0x61, 0x70, 0x5f, 0x62, 0x61, 0x73, 0x65, 0x03, 0x01,
        0x0a, 0x47, 0x01, 0x45, 0x01, 0x02, 0x7f, 0x02, 0x40, 0x20, 0x00, 0x42, 0x0f, 0x83, 0x42,
        0x03, 0x52, 0x20, 0x01, 0x42, 0x0f, 0x83, 0x42, 0x03, 0x52, 0x72, 0x45, 0x04, 0x40, 0x20,
        0x01, 0x42, 0x04, 0x88, 0xa7, 0x22, 0x02, 0x41, 0x00, 0x48, 0x20, 0x02, 0x20, 0x00, 0x42,
        0x04, 0x88, 0xa7, 0x22, 0x03, 0x6a, 0x22, 0x02, 0x20, 0x03, 0x48, 0x73, 0x45, 0x0d, 0x01,
        0x0b, 0x00, 0x0b, 0x20, 0x02, 0xad, 0x42, 0x04, 0x86, 0x42, 0x03, 0x84, 0x0b,
    ];
    let id: Hash = [0; 32].into();
    let vm = Vm::new(&host, id, &code)?;
    let a = 4i32;
    let b = 7i32;
    let c = 0x7fffffff_i32;
    let scvec0: ScVec = ScVec(vec![ScVal::I32(a), ScVal::I32(b)].try_into()?);
    let res = vm.invoke_function(&host, "add", &scvec0)?;
    match res {
        ScVal::I32(v) => assert_eq!(v, a + b),
        _ => panic!("Wrong result type"),
    }
    // overflow
    let scvec0: ScVec = ScVec(vec![ScVal::I32(a), ScVal::I32(c)].try_into()?);
    let res = vm.invoke_function(&host, "add", &scvec0);
    assert_matches!(res, Err(HostError::WASMI(wasmi::Error::Trap(_))));
    Ok(())
}

#[cfg(feature = "vm")]
#[test]
fn invoke_cross_contract() -> Result<(), HostError> {
    let contract_id: Hash = [0; 32].into();
    let key = ScVal::Static(ScStatic::LedgerKeyContractCodeWasm);
    let storage_key = LedgerKey::ContractData(LedgerKeyContractData {
        contract_id: contract_id.clone(),
        key: key.clone(),
    });
    let code: [u8; 163] = [
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x07, 0x01, 0x60, 0x02, 0x7e, 0x7e,
        0x01, 0x7e, 0x03, 0x02, 0x01, 0x00, 0x05, 0x03, 0x01, 0x00, 0x10, 0x06, 0x11, 0x02, 0x7f,
        0x00, 0x41, 0x80, 0x80, 0xc0, 0x00, 0x0b, 0x7f, 0x00, 0x41, 0x80, 0x80, 0xc0, 0x00, 0x0b,
        0x07, 0x2b, 0x04, 0x06, 0x6d, 0x65, 0x6d, 0x6f, 0x72, 0x79, 0x02, 0x00, 0x03, 0x61, 0x64,
        0x64, 0x00, 0x00, 0x0a, 0x5f, 0x5f, 0x64, 0x61, 0x74, 0x61, 0x5f, 0x65, 0x6e, 0x64, 0x03,
        0x00, 0x0b, 0x5f, 0x5f, 0x68, 0x65, 0x61, 0x70, 0x5f, 0x62, 0x61, 0x73, 0x65, 0x03, 0x01,
        0x0a, 0x47, 0x01, 0x45, 0x01, 0x02, 0x7f, 0x02, 0x40, 0x20, 0x00, 0x42, 0x0f, 0x83, 0x42,
        0x03, 0x52, 0x20, 0x01, 0x42, 0x0f, 0x83, 0x42, 0x03, 0x52, 0x72, 0x45, 0x04, 0x40, 0x20,
        0x01, 0x42, 0x04, 0x88, 0xa7, 0x22, 0x02, 0x41, 0x00, 0x48, 0x20, 0x02, 0x20, 0x00, 0x42,
        0x04, 0x88, 0xa7, 0x22, 0x03, 0x6a, 0x22, 0x02, 0x20, 0x03, 0x48, 0x73, 0x45, 0x0d, 0x01,
        0x0b, 0x00, 0x0b, 0x20, 0x02, 0xad, 0x42, 0x04, 0x86, 0x42, 0x03, 0x84, 0x0b,
    ];
    let scob = ScObject::Binary(code.try_into()?);
    let val = ScVal::Object(Some(scob));
    let le = LedgerEntry {
        last_modified_ledger_seq: 0,
        data: LedgerEntryData::ContractData(ContractDataEntry {
            contract_id,
            key,
            val,
        }),
        ext: LedgerEntryExt::V0,
    };
    let map = OrdMap::unit(storage_key.clone(), Some(le));
    let mut footprint = Footprint::default();
    footprint.record_access(&storage_key, AccessType::ReadOnly);

    // initialize storage and host
    let storage = Storage::with_enforcing_footprint_and_map(footprint, map);
    let host = Host::with_storage(storage);
    // create a dummy contract obj as the caller
    let scobj = ScObject::Binary([0; 32].try_into()?);
    let obj = host.to_host_obj(&scobj)?;
    // prepare arguments
    let sym = Symbol::from_str("add");
    let scvec0: ScVec = vec![ScVal::I32(1), ScVal::I32(2)].try_into()?;
    let args = host.to_host_obj(&ScObject::Vec(scvec0))?;

    let res = host.call(obj.to_object(), sym.into(), args.into())?;
    assert!(res.is::<i32>());
    assert!(res.get_tag() == Tag::I32);
    let i: i32 = res.try_into()?;
    assert_eq!(i, 3);
    Ok(())
}

#[cfg(feature = "vm")]
#[test]
fn invoke_cross_contract_with_err() -> Result<(), HostError> {
    let contract_id: Hash = [0; 32].into();
    let key = ScVal::Static(ScStatic::LedgerKeyContractCodeWasm);
    let storage_key = LedgerKey::ContractData(LedgerKeyContractData {
        contract_id: contract_id.clone(),
        key: key.clone(),
    });

    let code: [u8; 170] = [
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x11, 0x03, 0x60, 0x00, 0x01, 0x7e,
        0x60, 0x03, 0x7e, 0x7e, 0x7e, 0x01, 0x7e, 0x60, 0x01, 0x7e, 0x01, 0x7e, 0x02, 0x0d, 0x02,
        0x01, 0x76, 0x01, 0x5f, 0x00, 0x00, 0x01, 0x76, 0x01, 0x38, 0x00, 0x01, 0x03, 0x02, 0x01,
        0x02, 0x05, 0x03, 0x01, 0x00, 0x10, 0x06, 0x19, 0x03, 0x7f, 0x01, 0x41, 0x80, 0x80, 0xc0,
        0x00, 0x0b, 0x7f, 0x00, 0x41, 0x80, 0x80, 0xc0, 0x00, 0x0b, 0x7f, 0x00, 0x41, 0x80, 0x80,
        0xc0, 0x00, 0x0b, 0x07, 0x2f, 0x04, 0x06, 0x6d, 0x65, 0x6d, 0x6f, 0x72, 0x79, 0x02, 0x00,
        0x07, 0x76, 0x65, 0x63, 0x5f, 0x65, 0x72, 0x72, 0x00, 0x02, 0x0a, 0x5f, 0x5f, 0x64, 0x61,
        0x74, 0x61, 0x5f, 0x65, 0x6e, 0x64, 0x03, 0x01, 0x0b, 0x5f, 0x5f, 0x68, 0x65, 0x61, 0x70,
        0x5f, 0x62, 0x61, 0x73, 0x65, 0x03, 0x02, 0x0a, 0x29, 0x01, 0x27, 0x01, 0x01, 0x7f, 0x23,
        0x00, 0x41, 0x10, 0x6b, 0x22, 0x01, 0x24, 0x00, 0x20, 0x01, 0x10, 0x00, 0x42, 0xd1, 0x00,
        0x20, 0x00, 0x10, 0x01, 0x37, 0x03, 0x00, 0x20, 0x01, 0x29, 0x03, 0x00, 0x20, 0x01, 0x41,
        0x10, 0x6a, 0x24, 0x00, 0x0b,
    ];
    let scob = ScObject::Binary(code.try_into()?);
    let val = ScVal::Object(Some(scob));
    let le = LedgerEntry {
        last_modified_ledger_seq: 0,
        data: LedgerEntryData::ContractData(ContractDataEntry {
            contract_id,
            key,
            val,
        }),
        ext: LedgerEntryExt::V0,
    };
    let map = OrdMap::unit(storage_key.clone(), Some(le));

    let mut footprint = Footprint::default();
    footprint.record_access(&storage_key, AccessType::ReadOnly);

    // initialize storage and host
    let storage = Storage::with_enforcing_footprint_and_map(footprint, map);
    let host = Host::with_storage(storage);
    // create a dummy contract obj as the caller
    let scobj = ScObject::Binary([0; 32].try_into()?);
    let obj = host.to_host_obj(&scobj)?;
    // prepare arguments
    let sym = Symbol::from_str("vec_err");
    let scvec0: ScVec = vec![ScVal::I32(1)].try_into()?;
    let args = host.to_host_obj(&ScObject::Vec(scvec0))?;
    // call
    let sv = host.try_call(obj.to_object(), sym.into(), args.clone().into())?;
    let exp_st = Status::from_type_and_code(
        ScStatusType::HostObjectError,
        ScHostObjErrorCode::VecIndexOutOfBound as u32,
    );
    assert_eq!(sv.get_payload(), exp_st.to_raw().get_payload());
    assert_matches!(
        host.call(obj.to_object(), sym.into(), args.into()),
        Err(HostError::WASMI(wasmi::Error::Trap(wasmi::Trap::Host(_))))
    );
    Ok(())
}

#[cfg(feature = "vm")]
#[test]
fn invoke_cross_contract_lvl2_nested_with_err() -> Result<(), HostError> {
    // 1st level, the calling contract
    let id0: Hash = [0; 32].into();
    let key = ScVal::Static(ScStatic::LedgerKeyContractCodeWasm);
    let storage_key0 = LedgerKey::ContractData(LedgerKeyContractData {
        contract_id: id0.clone(),
        key: key.clone(),
    });
    let code0: [u8; 497] = [
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x17, 0x04, 0x60, 0x00, 0x01, 0x7e,
        0x60, 0x02, 0x7e, 0x7e, 0x01, 0x7e, 0x60, 0x03, 0x7e, 0x7e, 0x7e, 0x01, 0x7e, 0x60, 0x01,
        0x7e, 0x01, 0x7e, 0x02, 0x1f, 0x05, 0x01, 0x76, 0x01, 0x5f, 0x00, 0x00, 0x01, 0x76, 0x01,
        0x34, 0x00, 0x01, 0x01, 0x63, 0x01, 0x5f, 0x00, 0x02, 0x01, 0x62, 0x01, 0x33, 0x00, 0x00,
        0x01, 0x62, 0x01, 0x38, 0x00, 0x01, 0x03, 0x02, 0x01, 0x03, 0x05, 0x03, 0x01, 0x00, 0x11,
        0x06, 0x19, 0x03, 0x7f, 0x01, 0x41, 0x80, 0x80, 0xc0, 0x00, 0x0b, 0x7f, 0x00, 0x41, 0x87,
        0x80, 0xc0, 0x00, 0x0b, 0x7f, 0x00, 0x41, 0x90, 0x80, 0xc0, 0x00, 0x0b, 0x07, 0x30, 0x04,
        0x06, 0x6d, 0x65, 0x6d, 0x6f, 0x72, 0x79, 0x02, 0x00, 0x08, 0x64, 0x65, 0x6c, 0x5f, 0x63,
        0x61, 0x6c, 0x6c, 0x00, 0x05, 0x0a, 0x5f, 0x5f, 0x64, 0x61, 0x74, 0x61, 0x5f, 0x65, 0x6e,
        0x64, 0x03, 0x01, 0x0b, 0x5f, 0x5f, 0x68, 0x65, 0x61, 0x70, 0x5f, 0x62, 0x61, 0x73, 0x65,
        0x03, 0x02, 0x0a, 0xc4, 0x02, 0x01, 0xc1, 0x02, 0x02, 0x02, 0x7e, 0x04, 0x7f, 0x23, 0x00,
        0x41, 0x10, 0x6b, 0x22, 0x06, 0x24, 0x00, 0x41, 0x20, 0x21, 0x03, 0x10, 0x03, 0x21, 0x01,
        0x03, 0x7e, 0x20, 0x03, 0x04, 0x7e, 0x20, 0x03, 0x41, 0x01, 0x6b, 0x21, 0x03, 0x20, 0x01,
        0x42, 0x11, 0x10, 0x04, 0x21, 0x01, 0x0c, 0x01, 0x05, 0x20, 0x01, 0x02, 0x7e, 0x23, 0x00,
        0x41, 0x10, 0x6b, 0x22, 0x03, 0x24, 0x00, 0x03, 0x40, 0x02, 0x40, 0x02, 0x40, 0x20, 0x03,
        0x02, 0x7f, 0x20, 0x04, 0x41, 0x07, 0x46, 0x04, 0x40, 0x20, 0x03, 0x41, 0x08, 0x6a, 0x20,
        0x02, 0x42, 0x04, 0x86, 0x42, 0x09, 0x84, 0x37, 0x03, 0x00, 0x41, 0x00, 0x0c, 0x01, 0x0b,
        0x20, 0x04, 0x41, 0x0a, 0x47, 0x04, 0x40, 0x42, 0x01, 0x21, 0x01, 0x20, 0x04, 0x41, 0x80,
        0x80, 0x40, 0x6b, 0x2d, 0x00, 0x00, 0x22, 0x05, 0x41, 0xdf, 0x00, 0x46, 0x0d, 0x02, 0x20,
        0x05, 0xad, 0x21, 0x01, 0x02, 0x40, 0x02, 0x40, 0x20, 0x05, 0x41, 0x30, 0x6b, 0x41, 0xff,
        0x01, 0x71, 0x41, 0x0a, 0x4f, 0x04, 0x40, 0x20, 0x05, 0x41, 0xc1, 0x00, 0x6b, 0x41, 0xff,
        0x01, 0x71, 0x41, 0x1a, 0x49, 0x0d, 0x01, 0x20, 0x05, 0x41, 0xe1, 0x00, 0x6b, 0x41, 0xff,
        0x01, 0x71, 0x41, 0x1a, 0x49, 0x0d, 0x02, 0x20, 0x03, 0x41, 0x01, 0x36, 0x02, 0x04, 0x20,
        0x03, 0x41, 0x08, 0x6a, 0x20, 0x05, 0x36, 0x02, 0x00, 0x41, 0x01, 0x0c, 0x04, 0x0b, 0x20,
        0x01, 0x42, 0x2e, 0x7d, 0x21, 0x01, 0x0c, 0x04, 0x0b, 0x20, 0x01, 0x42, 0x35, 0x7d, 0x21,
        0x01, 0x0c, 0x03, 0x0b, 0x20, 0x01, 0x42, 0x3b, 0x7d, 0x21, 0x01, 0x0c, 0x02, 0x0b, 0x20,
        0x03, 0x41, 0x00, 0x36, 0x02, 0x04, 0x20, 0x03, 0x41, 0x08, 0x6a, 0x41, 0x07, 0x36, 0x02,
        0x00, 0x41, 0x01, 0x0b, 0x36, 0x02, 0x00, 0x0c, 0x01, 0x0b, 0x20, 0x04, 0x41, 0x01, 0x6a,
        0x21, 0x04, 0x20, 0x01, 0x20, 0x02, 0x42, 0x06, 0x86, 0x84, 0x21, 0x02, 0x0c, 0x01, 0x0b,
        0x0b, 0x20, 0x03, 0x28, 0x02, 0x00, 0x45, 0x04, 0x40, 0x20, 0x03, 0x29, 0x03, 0x08, 0x20,
        0x03, 0x41, 0x10, 0x6a, 0x24, 0x00, 0x0c, 0x01, 0x0b, 0x00, 0x0b, 0x10, 0x00, 0x20, 0x00,
        0x10, 0x01, 0x10, 0x02, 0x20, 0x06, 0x41, 0x10, 0x6a, 0x24, 0x00, 0x0b, 0x0b, 0x0b, 0x0b,
        0x10, 0x01, 0x00, 0x41, 0x80, 0x80, 0xc0, 0x00, 0x0b, 0x07, 0x76, 0x65, 0x63, 0x5f, 0x65,
        0x72, 0x72,
    ];
    let scob0 = ScObject::Binary(code0.try_into()?);
    let val0 = ScVal::Object(Some(scob0));
    let le0 = LedgerEntry {
        last_modified_ledger_seq: 0,
        data: LedgerEntryData::ContractData(ContractDataEntry {
            contract_id: id0,
            key: key.clone(),
            val: val0,
        }),
        ext: LedgerEntryExt::V0,
    };

    // 2nd level, the guest contract
    let id1: Hash = [1; 32].into();
    let storage_key1 = LedgerKey::ContractData(LedgerKeyContractData {
        contract_id: id1.clone(),
        key: key.clone(),
    });
    let code1: [u8; 170] = [
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x11, 0x03, 0x60, 0x00, 0x01, 0x7e,
        0x60, 0x03, 0x7e, 0x7e, 0x7e, 0x01, 0x7e, 0x60, 0x01, 0x7e, 0x01, 0x7e, 0x02, 0x0d, 0x02,
        0x01, 0x76, 0x01, 0x5f, 0x00, 0x00, 0x01, 0x76, 0x01, 0x38, 0x00, 0x01, 0x03, 0x02, 0x01,
        0x02, 0x05, 0x03, 0x01, 0x00, 0x10, 0x06, 0x19, 0x03, 0x7f, 0x01, 0x41, 0x80, 0x80, 0xc0,
        0x00, 0x0b, 0x7f, 0x00, 0x41, 0x80, 0x80, 0xc0, 0x00, 0x0b, 0x7f, 0x00, 0x41, 0x80, 0x80,
        0xc0, 0x00, 0x0b, 0x07, 0x2f, 0x04, 0x06, 0x6d, 0x65, 0x6d, 0x6f, 0x72, 0x79, 0x02, 0x00,
        0x07, 0x76, 0x65, 0x63, 0x5f, 0x65, 0x72, 0x72, 0x00, 0x02, 0x0a, 0x5f, 0x5f, 0x64, 0x61,
        0x74, 0x61, 0x5f, 0x65, 0x6e, 0x64, 0x03, 0x01, 0x0b, 0x5f, 0x5f, 0x68, 0x65, 0x61, 0x70,
        0x5f, 0x62, 0x61, 0x73, 0x65, 0x03, 0x02, 0x0a, 0x29, 0x01, 0x27, 0x01, 0x01, 0x7f, 0x23,
        0x00, 0x41, 0x10, 0x6b, 0x22, 0x01, 0x24, 0x00, 0x20, 0x01, 0x10, 0x00, 0x42, 0xd1, 0x00,
        0x20, 0x00, 0x10, 0x01, 0x37, 0x03, 0x00, 0x20, 0x01, 0x29, 0x03, 0x00, 0x20, 0x01, 0x41,
        0x10, 0x6a, 0x24, 0x00, 0x0b,
    ];
    let scob1 = ScObject::Binary(code1.try_into()?);
    let val1 = ScVal::Object(Some(scob1));
    let le1 = LedgerEntry {
        last_modified_ledger_seq: 0,
        data: LedgerEntryData::ContractData(ContractDataEntry {
            contract_id: id1,
            key,
            val: val1,
        }),
        ext: LedgerEntryExt::V0,
    };

    // create storage map and footprint
    let mut map = OrdMap::unit(storage_key0.clone(), Some(le0));
    map.insert(storage_key1.clone(), Some(le1));
    let mut footprint = Footprint::default();
    footprint.record_access(&storage_key0, AccessType::ReadOnly);
    footprint.record_access(&storage_key1, AccessType::ReadOnly);

    // initialize storage and host
    let storage = Storage::with_enforcing_footprint_and_map(footprint, map);
    let host = Host::with_storage(storage);
    // prepare arguments
    let scobj = ScObject::Binary([0; 32].try_into()?);
    let obj = host.to_host_obj(&scobj)?;
    let sym = Symbol::from_str("del_call");
    let scvec0: ScVec = vec![ScVal::I32(1)].try_into()?;
    let args = host.to_host_obj(&ScObject::Vec(scvec0))?;
    // try call
    let sv = host.try_call(obj.to_object(), sym.into(), args.clone().into())?;
    let exp_st = Status::from_status(ScStatus::HostObjectError(
        ScHostObjErrorCode::VecIndexOutOfBound,
    ));
    assert_eq!(sv.get_payload(), exp_st.as_ref().get_payload());
    // call
    assert_matches!(
        host.call(obj.to_object(), sym.into(), args.into()),
        Err(HostError::WASMI(wasmi::Error::Trap(wasmi::Trap::Host(_))))
    );
    Ok(())
}

#[test]
fn binary_suite_of_tests() -> Result<(), HostError> {
    let host = Host::default();
    // new and push
    let mut obj = host.binary_new()?;
    for i in 0..32 {
        obj = host.binary_push(obj, (i as u32).into())?;
    }
    if let ScObject::Binary(b) = host.from_host_obj(obj)? {
        assert_eq!((0..32).collect::<Vec<u8>>().as_slice(), b.as_slice());
    } else {
        return Err(HostError::General("Type error"));
    }
    // pop and len
    for _ in 0..24 {
        obj = host.binary_pop(obj)?;
    }
    assert_eq!(
        unsafe { <u32 as RawValConvertible>::unchecked_from_val(host.binary_len(obj)?) },
        8
    );
    assert_eq!(
        unsafe {
            <u8 as RawValConvertible>::unchecked_from_val(host.binary_get(obj, 5_u32.into())?)
        },
        5
    );
    // put, del, get, front, back
    obj = host.binary_put(obj, 5_u32.into(), 99_u32.into())?;
    assert_eq!(
        unsafe {
            <u8 as RawValConvertible>::unchecked_from_val(host.binary_get(obj, 5_u32.into())?)
        },
        99
    );
    obj = host.binary_del(obj, 5_u32.into())?; // [0,1,2,3,4,6,7]
    assert_eq!(
        unsafe { <u32 as RawValConvertible>::unchecked_from_val(host.binary_len(obj)?) },
        7
    );
    assert_eq!(
        unsafe { <u32 as RawValConvertible>::unchecked_from_val(host.binary_front(obj)?) },
        0
    );
    assert_eq!(
        unsafe { <u32 as RawValConvertible>::unchecked_from_val(host.binary_back(obj)?) },
        7
    );
    // insert, slice and append
    obj = host.binary_insert(obj, 5_u32.into(), 5_u32.into())?; // [0,1,2,3,4,5,6,7]
    let obj0 = host.binary_slice(obj, 0_u32.into(), 3_u32.into())?; // [0,1,2]
    if let ScObject::Binary(b) = host.from_host_obj(obj0)? {
        assert_eq!((0..3).collect::<Vec<u8>>().as_slice(), b.as_slice());
    } else {
        return Err(HostError::General("Type error"));
    }
    let obj1 = host.binary_slice(obj, 3_u32.into(), 5_u32.into())?; // [3,4,5,6,7]
    if let ScObject::Binary(b) = host.from_host_obj(obj1)? {
        assert_eq!((3..8).collect::<Vec<u8>>().as_slice(), b.as_slice());
    } else {
        return Err(HostError::General("Type error"));
    }
    let obj_back = host.binary_append(obj0, obj1)?;
    assert_eq!(host.obj_cmp(obj.into(), obj_back.into())?, 0);

    Ok(())
}

#[test]
fn binary_xdr_roundtrip() -> Result<(), HostError> {
    let host = Host::default();
    let scv: ScVec = vec![ScVal::U32(1), ScVal::U32(2)].try_into()?;
    let sco = ScObject::Vec(scv);
    let obj = host.to_host_obj(&sco)?;
    let bo = host.serialize_to_binary(obj.clone().into())?;
    let obj_back = host.deserialize_from_binary(bo)?;
    assert_eq!(host.obj_cmp(obj.into(), obj_back.into())?, 0);
    Ok(())
}

#[test]
fn bigint_tests() -> Result<(), HostError> {
    let host = Host::default();
    let a: u64 = 2374340;
    let b: i64 = -438730;
    let obj_0 = host.bigint_from_i64(0)?;
    let obj_a = host.bigint_from_u64(a)?;
    let obj_b = host.bigint_from_i64(b)?;
    // add
    {
        let obj_res = host.bigint_add(obj_a, obj_b)?;
        let obj_ref = host.bigint_from_i64(a as i64 + b)?;
        assert_eq!(host.obj_cmp(obj_res.into(), obj_ref.into())?, 0);
    }
    // sub
    {
        let obj_res = host.bigint_sub(obj_a, obj_b)?;
        let obj_ref = host.bigint_from_i64(a as i64 - b)?;
        assert_eq!(host.obj_cmp(obj_res.into(), obj_ref.into())?, 0);
    }
    // mul
    {
        let obj_res = host.bigint_mul(obj_a, obj_b)?;
        let obj_ref = host.bigint_from_i64(a as i64 * b)?;
        assert_eq!(host.obj_cmp(obj_res.into(), obj_ref.into())?, 0);
    }
    // div
    {
        let obj_res = host.bigint_div(obj_a, obj_b)?;
        let obj_ref = host.bigint_from_i64(a as i64 / b)?;
        assert_eq!(host.obj_cmp(obj_res.into(), obj_ref.into())?, 0);
        // div by 0
        assert_matches!(
            host.bigint_div(obj_a, obj_0),
            Err(HostError::General("bigint division by zero"))
        );
    }
    // rem
    {
        let obj_res = host.bigint_rem(obj_a, obj_b)?;
        let obj_ref = host.bigint_from_i64(a as i64 % b)?;
        assert_eq!(host.obj_cmp(obj_res.into(), obj_ref.into())?, 0);
        // div by 0
        assert_matches!(
            host.bigint_rem(obj_a, obj_0),
            Err(HostError::General("bigint division by zero"))
        );
    }
    // and
    {
        let obj_res = host.bigint_and(obj_a, obj_b)?;
        let obj_ref = host.bigint_from_i64(a as i64 & b)?;
        assert_eq!(host.obj_cmp(obj_res.into(), obj_ref.into())?, 0);
    }
    // or
    {
        let obj_res = host.bigint_or(obj_a, obj_b)?;
        let obj_ref = host.bigint_from_i64(a as i64 | b)?;
        assert_eq!(host.obj_cmp(obj_res.into(), obj_ref.into())?, 0);
    }
    // xor
    {
        let obj_res = host.bigint_xor(obj_a, obj_b)?;
        let obj_ref = host.bigint_from_i64(a as i64 ^ b)?;
        assert_eq!(host.obj_cmp(obj_res.into(), obj_ref.into())?, 0);
    }
    // shl
    {
        let obj_res = host.bigint_shl(obj_a, host.bigint_from_i64(5)?)?;
        let obj_ref = host.bigint_from_u64(a << 5)?;
        assert_eq!(host.obj_cmp(obj_res.into(), obj_ref.into())?, 0);
        assert_matches!(
            host.bigint_shl(obj_a, host.bigint_from_i64(-5)?),
            Err(HostError::General("attempt to shift left with negative"))
        );
        // a 65-bit integer
        let obj_c = host.bigint_shl(host.bigint_from_u64(u64::MAX)?, host.bigint_from_i64(1)?)?;
        assert_matches!(
            host.bigint_shl(obj_a, obj_c),
            Err(HostError::General("left-shift overflow"))
        );
    }
    // shr
    {
        let obj_res = host.bigint_shr(obj_a, host.bigint_from_i64(5)?)?;
        let obj_ref = host.bigint_from_u64(a >> 5)?;
        assert_eq!(host.obj_cmp(obj_res.into(), obj_ref.into())?, 0);
        assert_matches!(
            host.bigint_shr(obj_a, host.bigint_from_i64(-5)?),
            Err(HostError::General("attempt to shift right with negative"))
        );
        // a 65-bit integer
        let obj_c = host.bigint_shl(host.bigint_from_u64(u64::MAX)?, host.bigint_from_i64(1)?)?;
        assert_matches!(
            host.bigint_shr(obj_a, obj_c),
            Err(HostError::General("right-shift overflow"))
        );
    }
    // cmp
    {
        use std::cmp::Ordering;
        let ord_greater: i32 = host.bigint_cmp(obj_a, obj_b)?.try_into()?;
        let ord_less: i32 = host.bigint_cmp(obj_b, obj_a)?.try_into()?;
        let obj3 = host.bigint_from_u64(a)?;
        let ord_equal: i32 = host.bigint_cmp(obj_a, obj3)?.try_into()?;
        assert_eq!(ord_greater, Ordering::Greater as i32);
        assert_eq!(ord_less, Ordering::Less as i32);
        assert_eq!(ord_equal, Ordering::Equal as i32);
    }
    // is zero
    {
        let f = RawVal::from_bool(false);
        let t = RawVal::from_bool(true);
        assert_eq!(host.bigint_is_zero(obj_a)?.get_payload(), f.get_payload());
        assert_eq!(host.bigint_is_zero(obj_b)?.get_payload(), f.get_payload());
        assert_eq!(host.bigint_is_zero(obj_0)?.get_payload(), t.get_payload());
    }
    // neg
    {
        let obj_res = host.bigint_neg(obj_b)?;
        let obj_ref = host.bigint_from_i64(-b)?;
        assert_eq!(host.obj_cmp(obj_res.into(), obj_ref.into())?, 0);
        assert_eq!(
            host.obj_cmp(host.bigint_neg(obj_res)?.into(), obj_b.into())?,
            0
        );
    }
    // not
    {
        let obj_res = host.bigint_not(obj_b)?;
        let obj_ref = host.bigint_from_i64(!b)?;
        assert_eq!(host.obj_cmp(obj_res.into(), obj_ref.into())?, 0);
        assert_eq!(
            host.obj_cmp(host.bigint_not(obj_res)?.into(), obj_b.into())?,
            0
        );
    }
    // gcd
    {
        let obj_res = host.bigint_gcd(obj_a, obj_b)?;
        let obj_ref = host.bigint_from_i64(10)?;
        assert_eq!(host.obj_cmp(obj_res.into(), obj_ref.into())?, 0);
        // gcd by 0 is self
        let obj_res = host.bigint_gcd(obj_a, obj_0)?;
        assert_eq!(host.obj_cmp(obj_res.into(), obj_a.into())?, 0);
        // gcd of (0, 0) is 0
        let obj_res = host.bigint_gcd(obj_0, obj_0)?;
        assert_eq!(host.obj_cmp(obj_res.into(), obj_0.into())?, 0);
    }
    // lcm
    {
        let obj_res = host.bigint_lcm(obj_a, obj_b)?;
        let obj_ref = host.bigint_from_i64(104169418820)?;
        assert_eq!(host.obj_cmp(obj_res.into(), obj_ref.into())?, 0);
        // lcm by 0 is 0
        let obj_res = host.bigint_lcm(obj_a, obj_0)?;
        assert_eq!(host.obj_cmp(obj_res.into(), obj_0.into())?, 0);
        // lcm of (0, 0) is 0
        let obj_res = host.bigint_lcm(obj_0, obj_0)?;
        assert_eq!(host.obj_cmp(obj_res.into(), obj_0.into())?, 0);
    }
    // pow
    {
        let obj_res = host.bigint_pow(obj_b, host.bigint_from_u64(2_u32.into())?)?;
        let obj_ref = host.bigint_from_i64(192484012900)?;
        assert_eq!(host.obj_cmp(obj_res.into(), obj_ref.into())?, 0);
        let obj_res = host.bigint_pow(obj_b, host.bigint_from_u64(0_u32.into())?)?;
        let obj_ref = host.bigint_from_i64(1)?;
        assert_eq!(host.obj_cmp(obj_res.into(), obj_ref.into())?, 0);
        assert_matches!(
            host.bigint_pow(obj_b, host.bigint_from_i64(-1)?),
            Err(HostError::General("negative exponentiation not supported"))
        );
        // a 65-bit integer
        let obj_c = host.bigint_shl(host.bigint_from_u64(u64::MAX)?, host.bigint_from_i64(1)?)?;
        assert_matches!(
            host.bigint_pow(obj_b, obj_c),
            Err(HostError::General("pow overflow"))
        );
    }
    // pow_mod
    {
        let obj_2 = host.bigint_from_i64(2)?;
        let obj_res = host.bigint_pow_mod(obj_a, obj_2, obj_b)?;
        let obj_ref = host.bigint_from_i64(-94310)?;
        assert_eq!(host.obj_cmp(obj_res.into(), obj_ref.into())?, 0);

        assert_matches!(
            host.bigint_pow_mod(obj_a, obj_b, obj_2),
            Err(HostError::General("negative exponentiation not supported"))
        );
        assert_matches!(
            host.bigint_pow_mod(obj_a, obj_2, obj_0),
            Err(HostError::General("zero modulus not supported"))
        );
    }
    // sqrt
    {
        let obj_res = host.bigint_sqrt(obj_a)?;
        let obj_ref = host.bigint_from_i64(1540)?;
        assert_eq!(host.obj_cmp(obj_res.into(), obj_ref.into())?, 0);
        assert_matches!(
            host.bigint_sqrt(obj_b),
            Err(HostError::General("sqrt is imaginary"))
        );
    }
    // bits
    {
        assert_eq!(host.bigint_bits(obj_a)?, 22);
        assert_eq!(host.bigint_bits(obj_b)?, 19);
        assert_eq!(host.bigint_bits(obj_0)?, 0);
    }

    Ok(())
}

#[test]
fn test_conversion_error_with_tuple() -> Result<(), HostError> {
    let host = Host::default();
    let scvec: ScVec = vec![ScVal::U32(2)].try_into()?;
    let rv = host.to_host_obj(&ScObject::Vec(scvec))?.to_raw();
    let ev = EnvVal { env: host, val: rv };
    assert_matches!(
        TryInto::<(u32, u32)>::try_into(ev).map_err(|e| e.into()),
        Err(HostError::ConversionError(_))
    );
    Ok(())
}
