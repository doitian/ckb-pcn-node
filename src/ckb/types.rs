use super::gen::pcn::{self as molecule_pcn, Byte66, SignatureVec};
use super::serde_utils::{EntityWrapperBase64, WrapperHex};
use ckb_types::{
    packed::{Byte32 as MByte32, BytesVec, Script, Transaction},
    prelude::{Pack, Unpack},
};
use molecule::prelude::{Builder, Byte, Entity};
use musig2::errors::DecodeError;
use musig2::{BinaryEncoding, PubNonce};
use once_cell::sync::OnceCell;
use secp256k1::{ecdsa::Signature as Secp256k1Signature, All, PublicKey, Secp256k1, SecretKey};
use serde::{Deserialize, Serialize};
use serde_with::base64::Base64;
use serde_with::serde_as;
use thiserror::Error;

pub fn secp256k1_instance() -> &'static Secp256k1<All> {
    static INSTANCE: OnceCell<Secp256k1<All>> = OnceCell::new();
    INSTANCE.get_or_init(|| Secp256k1::new())
}

impl From<&Byte66> for PubNonce {
    fn from(value: &Byte66) -> Self {
        PubNonce::from_bytes(value.as_slice()).unwrap()
    }
}

impl From<&PubNonce> for Byte66 {
    fn from(value: &PubNonce) -> Self {
        Self::new_builder()
            .set(
                value
                    .to_bytes()
                    .into_iter()
                    .map(Byte::new)
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap(),
            )
            .build()
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Debug)]
pub struct Privkey(pub SecretKey);

impl From<Privkey> for SecretKey {
    fn from(pk: Privkey) -> Self {
        pk.0
    }
}

impl From<SecretKey> for Privkey {
    fn from(sk: SecretKey) -> Self {
        Self(sk)
    }
}

impl From<&[u8; 32]> for Privkey {
    fn from(k: &[u8; 32]) -> Self {
        Self::from_slice(k)
    }
}

impl AsRef<[u8; 32]> for Privkey {
    /// Gets a reference to the underlying array.
    ///
    /// # Side channel attacks
    ///
    /// Using ordering functions (`PartialOrd`/`Ord`) on a reference to secret keys leaks data
    /// because the implementations are not constant time. Doing so will make your code vulnerable
    /// to side channel attacks. [`SecretKey::eq`] is implemented using a constant time algorithm,
    /// please consider using it to do comparisons of secret keys.
    #[inline]
    fn as_ref(&self) -> &[u8; 32] {
        self.0.as_ref()
    }
}

#[serde_as]
#[derive(Copy, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct Hash256(#[serde_as(as = "WrapperHex<[u8; 32]>")] [u8; 32]);

impl From<[u8; 32]> for Hash256 {
    fn from(value: [u8; 32]) -> Self {
        Self(value)
    }
}

impl From<&Hash256> for MByte32 {
    fn from(hash: &Hash256) -> Self {
        MByte32::new_builder()
            .set(
                hash.0
                    .into_iter()
                    .map(Byte::new)
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap(),
            )
            .build()
    }
}

impl From<Hash256> for MByte32 {
    fn from(hash: Hash256) -> Self {
        (&hash).into()
    }
}

impl From<&MByte32> for Hash256 {
    fn from(value: &MByte32) -> Self {
        Hash256(value.as_bytes().to_vec().try_into().unwrap())
    }
}

impl From<MByte32> for Hash256 {
    fn from(value: MByte32) -> Self {
        (&value).into()
    }
}

impl ::core::fmt::LowerHex for Hash256 {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        if f.alternate() {
            write!(f, "0x")?;
        }
        write!(f, "{}", hex::encode(self.0))
    }
}

impl ::core::fmt::Debug for Hash256 {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        write!(f, "{}({:#x})", "Hash256", self)
    }
}

impl ::core::fmt::Display for Hash256 {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        let raw_data = hex::encode(&self.0);
        write!(f, "{}(0x{})", "Hash256", raw_data)
    }
}

impl ::core::default::Default for Hash256 {
    fn default() -> Self {
        Self([0; 32])
    }
}

impl Privkey {
    pub fn from_slice(key: &[u8]) -> Self {
        SecretKey::from_slice(key)
            .expect("Invalid secret key")
            .into()
    }

    pub fn pubkey(&self) -> Pubkey {
        Pubkey::from(self.0.public_key(secp256k1_instance()))
    }
}

#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Pubkey(pub PublicKey);

impl From<Pubkey> for PublicKey {
    fn from(pk: Pubkey) -> Self {
        pk.0
    }
}

impl From<PublicKey> for Pubkey {
    fn from(pk: PublicKey) -> Pubkey {
        Pubkey(pk)
    }
}

#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct Signature(pub Secp256k1Signature);

impl From<Signature> for Secp256k1Signature {
    fn from(sig: Signature) -> Self {
        sig.0
    }
}

impl From<Secp256k1Signature> for Signature {
    fn from(sig: Secp256k1Signature) -> Self {
        Self(sig)
    }
}

/// The error type wrap various ser/de errors.
#[derive(Error, Debug)]
pub enum Error {
    /// Invalid pubkey/signature format
    #[error("Secp error: {0}")]
    Secp(#[from] secp256k1::Error),
    #[error("Molecule error: {0}")]
    Molecule(#[from] molecule::error::VerificationError),
    #[error("Musig2 error: {0}")]
    Musig2(String),
}

impl From<Pubkey> for molecule_pcn::Pubkey {
    fn from(pk: Pubkey) -> molecule_pcn::Pubkey {
        molecule_pcn::Pubkey::new_builder()
            .set(
                pk.0.serialize()
                    .into_iter()
                    .map(Into::into)
                    .collect::<Vec<Byte>>()
                    .try_into()
                    .expect("Public serialized to corrent length"),
            )
            .build()
    }
}

impl TryFrom<molecule_pcn::Pubkey> for Pubkey {
    type Error = Error;

    fn try_from(pubkey: molecule_pcn::Pubkey) -> Result<Self, Self::Error> {
        let pubkey = pubkey.as_slice();
        PublicKey::from_slice(pubkey)
            .map(Into::into)
            .map_err(Into::into)
    }
}

impl From<Signature> for molecule_pcn::Signature {
    fn from(signature: Signature) -> molecule_pcn::Signature {
        molecule_pcn::Signature::new_builder()
            .set(
                signature
                    .0
                    .serialize_compact()
                    .into_iter()
                    .map(Into::into)
                    .collect::<Vec<Byte>>()
                    .try_into()
                    .expect("Signature serialized to corrent length"),
            )
            .build()
    }
}

impl TryFrom<molecule_pcn::Signature> for Signature {
    type Error = Error;

    fn try_from(signature: molecule_pcn::Signature) -> Result<Self, Self::Error> {
        let signature = signature.as_slice();
        Secp256k1Signature::from_compact(signature)
            .map(Into::into)
            .map_err(Into::into)
    }
}

impl TryFrom<Byte66> for PubNonce {
    type Error = DecodeError<Self>;

    fn try_from(value: Byte66) -> Result<Self, Self::Error> {
        PubNonce::from_bytes(value.as_slice())
    }
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenChannel {
    pub chain_hash: Hash256,
    pub channel_id: Hash256,
    #[serde_as(as = "Option<EntityWrapperBase64<Script>>")]
    pub funding_type_script: Option<Script>,
    pub funding_amount: u64,
    pub funding_fee_rate: u64,
    pub commitment_fee_rate: u64,
    pub max_tlc_value_in_flight: u64,
    pub max_accept_tlcs: u64,
    pub min_tlc_value: u64,
    pub to_self_delay: u64,
    pub funding_pubkey: Pubkey,
    pub revocation_basepoint: Pubkey,
    pub payment_basepoint: Pubkey,
    pub delayed_payment_basepoint: Pubkey,
    pub tlc_basepoint: Pubkey,
    pub first_per_commitment_point: Pubkey,
    pub second_per_commitment_point: Pubkey,
    pub next_local_nonce: PubNonce,
    pub channel_flags: u8,
}

impl From<OpenChannel> for molecule_pcn::OpenChannel {
    fn from(open_channel: OpenChannel) -> Self {
        molecule_pcn::OpenChannel::new_builder()
            .chain_hash(open_channel.chain_hash.into())
            .channel_id(open_channel.channel_id.into())
            .funding_type_script(open_channel.funding_type_script.pack())
            .funding_amount(open_channel.funding_amount.pack())
            .funding_fee_rate(open_channel.funding_fee_rate.pack())
            .commitment_fee_rate(open_channel.commitment_fee_rate.pack())
            .max_tlc_value_in_flight(open_channel.max_tlc_value_in_flight.pack())
            .max_accept_tlcs(open_channel.max_accept_tlcs.pack())
            .min_tlc_value(open_channel.min_tlc_value.pack())
            .to_self_delay(open_channel.to_self_delay.pack())
            .funding_pubkey(open_channel.funding_pubkey.into())
            .revocation_basepoint(open_channel.revocation_basepoint.into())
            .payment_basepoint(open_channel.payment_basepoint.into())
            .delayed_payment_basepoint(open_channel.delayed_payment_basepoint.into())
            .tlc_basepoint(open_channel.tlc_basepoint.into())
            .first_per_commitment_point(open_channel.first_per_commitment_point.into())
            .second_per_commitment_point(open_channel.second_per_commitment_point.into())
            .next_local_nonce((&open_channel.next_local_nonce).into())
            .channel_flags(open_channel.channel_flags.into())
            .build()
    }
}

impl TryFrom<molecule_pcn::OpenChannel> for OpenChannel {
    type Error = Error;

    fn try_from(open_channel: molecule_pcn::OpenChannel) -> Result<Self, Self::Error> {
        Ok(OpenChannel {
            chain_hash: open_channel.chain_hash().into(),
            channel_id: open_channel.channel_id().into(),
            funding_type_script: open_channel.funding_type_script().to_opt(),
            funding_amount: open_channel.funding_amount().unpack(),
            funding_fee_rate: open_channel.funding_fee_rate().unpack(),
            commitment_fee_rate: open_channel.commitment_fee_rate().unpack(),
            max_tlc_value_in_flight: open_channel.max_tlc_value_in_flight().unpack(),
            max_accept_tlcs: open_channel.max_accept_tlcs().unpack(),
            min_tlc_value: open_channel.min_tlc_value().unpack(),
            to_self_delay: open_channel.to_self_delay().unpack(),
            funding_pubkey: open_channel.funding_pubkey().try_into()?,
            revocation_basepoint: open_channel.revocation_basepoint().try_into()?,
            payment_basepoint: open_channel.payment_basepoint().try_into()?,
            delayed_payment_basepoint: open_channel.delayed_payment_basepoint().try_into()?,
            tlc_basepoint: open_channel.tlc_basepoint().try_into()?,
            first_per_commitment_point: open_channel.first_per_commitment_point().try_into()?,
            second_per_commitment_point: open_channel.second_per_commitment_point().try_into()?,
            next_local_nonce: open_channel
                .next_local_nonce()
                .try_into()
                .map_err(|err| Error::Musig2(format!("{err}")))?,
            channel_flags: open_channel.channel_flags().into(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptChannel {
    pub channel_id: Hash256,
    // TODO: remove funding_amount.
    // https://github.com/lightning/bolts/blob/master/02-peer-protocol.md#the-accept_channel-message
    // bolts2 does not mention the funding_satoshis in accept channel.
    // Maybe we should remove it.
    pub funding_amount: u64,
    pub max_tlc_value_in_flight: u64,
    pub max_accept_tlcs: u64,
    pub min_tlc_value: u64,
    pub to_self_delay: u64,
    pub funding_pubkey: Pubkey,
    pub revocation_basepoint: Pubkey,
    pub payment_basepoint: Pubkey,
    pub delayed_payment_basepoint: Pubkey,
    pub tlc_basepoint: Pubkey,
    pub first_per_commitment_point: Pubkey,
    pub second_per_commitment_point: Pubkey,
    pub next_local_nonce: PubNonce,
}

impl From<AcceptChannel> for molecule_pcn::AcceptChannel {
    fn from(accept_channel: AcceptChannel) -> Self {
        molecule_pcn::AcceptChannel::new_builder()
            .channel_id(accept_channel.channel_id.into())
            .funding_amount(accept_channel.funding_amount.pack())
            .max_tlc_value_in_flight(accept_channel.max_tlc_value_in_flight.pack())
            .max_accept_tlcs(accept_channel.max_accept_tlcs.pack())
            .min_tlc_value(accept_channel.min_tlc_value.pack())
            .to_self_delay(accept_channel.to_self_delay.pack())
            .funding_pubkey(accept_channel.funding_pubkey.into())
            .revocation_basepoint(accept_channel.revocation_basepoint.into())
            .payment_basepoint(accept_channel.payment_basepoint.into())
            .delayed_payment_basepoint(accept_channel.delayed_payment_basepoint.into())
            .tlc_basepoint(accept_channel.tlc_basepoint.into())
            .first_per_commitment_point(accept_channel.first_per_commitment_point.into())
            .second_per_commitment_point(accept_channel.second_per_commitment_point.into())
            .next_local_nonce((&accept_channel.next_local_nonce).into())
            .build()
    }
}

impl TryFrom<molecule_pcn::AcceptChannel> for AcceptChannel {
    type Error = Error;

    fn try_from(accept_channel: molecule_pcn::AcceptChannel) -> Result<Self, Self::Error> {
        Ok(AcceptChannel {
            channel_id: accept_channel.channel_id().into(),
            funding_amount: accept_channel.funding_amount().unpack(),
            max_tlc_value_in_flight: accept_channel.max_tlc_value_in_flight().unpack(),
            max_accept_tlcs: accept_channel.max_accept_tlcs().unpack(),
            min_tlc_value: accept_channel.min_tlc_value().unpack(),
            to_self_delay: accept_channel.to_self_delay().unpack(),
            funding_pubkey: accept_channel.funding_pubkey().try_into()?,
            revocation_basepoint: accept_channel.revocation_basepoint().try_into()?,
            payment_basepoint: accept_channel.payment_basepoint().try_into()?,
            delayed_payment_basepoint: accept_channel.delayed_payment_basepoint().try_into()?,
            tlc_basepoint: accept_channel.tlc_basepoint().try_into()?,
            first_per_commitment_point: accept_channel.first_per_commitment_point().try_into()?,
            second_per_commitment_point: accept_channel.second_per_commitment_point().try_into()?,
            next_local_nonce: accept_channel
                .next_local_nonce()
                .try_into()
                .map_err(|err| Error::Musig2(format!("{err}")))?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentSigned {
    pub channel_id: Hash256,
    pub signature: Signature,
}

impl From<CommitmentSigned> for molecule_pcn::CommitmentSigned {
    fn from(commitment_signed: CommitmentSigned) -> Self {
        molecule_pcn::CommitmentSigned::new_builder()
            .channel_id(commitment_signed.channel_id.into())
            .signature(commitment_signed.signature.into())
            .build()
    }
}

impl TryFrom<molecule_pcn::CommitmentSigned> for CommitmentSigned {
    type Error = Error;

    fn try_from(commitment_signed: molecule_pcn::CommitmentSigned) -> Result<Self, Self::Error> {
        Ok(CommitmentSigned {
            channel_id: commitment_signed.channel_id().into(),
            signature: commitment_signed.signature().try_into()?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxSignatures {
    pub channel_id: Hash256,
    pub tx_hash: Hash256,
    pub witnesses: Vec<Vec<u8>>,
}

impl From<TxSignatures> for molecule_pcn::TxSignatures {
    fn from(tx_signatures: TxSignatures) -> Self {
        molecule_pcn::TxSignatures::new_builder()
            .channel_id(tx_signatures.channel_id.into())
            .tx_hash(tx_signatures.tx_hash.into())
            .witnesses(
                BytesVec::new_builder()
                    .set(
                        tx_signatures
                            .witnesses
                            .into_iter()
                            .map(|witness| witness.pack())
                            .collect(),
                    )
                    .build(),
            )
            .build()
    }
}

impl TryFrom<molecule_pcn::TxSignatures> for TxSignatures {
    type Error = Error;

    fn try_from(tx_signatures: molecule_pcn::TxSignatures) -> Result<Self, Self::Error> {
        Ok(TxSignatures {
            channel_id: tx_signatures.channel_id().into(),
            tx_hash: tx_signatures.tx_hash().into(),
            witnesses: tx_signatures
                .witnesses()
                .into_iter()
                .map(|witness| witness.unpack())
                .collect(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelReady {
    pub channel_id: Hash256,
}

impl From<ChannelReady> for molecule_pcn::ChannelReady {
    fn from(channel_ready: ChannelReady) -> Self {
        molecule_pcn::ChannelReady::new_builder()
            .channel_id(channel_ready.channel_id.into())
            .build()
    }
}

impl TryFrom<molecule_pcn::ChannelReady> for ChannelReady {
    type Error = Error;

    fn try_from(channel_ready: molecule_pcn::ChannelReady) -> Result<Self, Self::Error> {
        Ok(ChannelReady {
            channel_id: channel_ready.channel_id().into(),
        })
    }
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxAdd {
    pub channel_id: Hash256,
    #[serde_as(as = "EntityWrapperBase64<Transaction>")]
    pub tx: Transaction,
}

impl From<TxAdd> for molecule_pcn::TxAdd {
    fn from(tx_add: TxAdd) -> Self {
        molecule_pcn::TxAdd::new_builder()
            .channel_id(tx_add.channel_id.into())
            .tx(tx_add.tx)
            .build()
    }
}

impl TryFrom<molecule_pcn::TxAdd> for TxAdd {
    type Error = Error;

    fn try_from(tx_add: molecule_pcn::TxAdd) -> Result<Self, Self::Error> {
        Ok(TxAdd {
            channel_id: tx_add.channel_id().into(),
            tx: tx_add.tx(),
        })
    }
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxRemove {
    pub channel_id: Hash256,
    #[serde_as(as = "EntityWrapperBase64<Transaction>")]
    pub tx: Transaction,
}

impl From<TxRemove> for molecule_pcn::TxRemove {
    fn from(tx_remove: TxRemove) -> Self {
        molecule_pcn::TxRemove::new_builder()
            .channel_id(tx_remove.channel_id.into())
            .tx(tx_remove.tx)
            .build()
    }
}

impl TryFrom<molecule_pcn::TxRemove> for TxRemove {
    type Error = Error;

    fn try_from(tx_remove: molecule_pcn::TxRemove) -> Result<Self, Self::Error> {
        Ok(TxRemove {
            channel_id: tx_remove.channel_id().into(),
            tx: tx_remove.tx(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxComplete {
    pub channel_id: Hash256,
}

impl From<TxComplete> for molecule_pcn::TxComplete {
    fn from(tx_complete: TxComplete) -> Self {
        molecule_pcn::TxComplete::new_builder()
            .channel_id(tx_complete.channel_id.into())
            .build()
    }
}

impl TryFrom<molecule_pcn::TxComplete> for TxComplete {
    type Error = Error;

    fn try_from(tx_complete: molecule_pcn::TxComplete) -> Result<Self, Self::Error> {
        Ok(TxComplete {
            channel_id: tx_complete.channel_id().into(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxAbort {
    pub channel_id: Hash256,
    pub message: Vec<u8>,
}

impl From<TxAbort> for molecule_pcn::TxAbort {
    fn from(tx_abort: TxAbort) -> Self {
        molecule_pcn::TxAbort::new_builder()
            .channel_id(tx_abort.channel_id.into())
            .message(tx_abort.message.pack())
            .build()
    }
}

impl TryFrom<molecule_pcn::TxAbort> for TxAbort {
    type Error = Error;

    fn try_from(tx_abort: molecule_pcn::TxAbort) -> Result<Self, Self::Error> {
        Ok(TxAbort {
            channel_id: tx_abort.channel_id().into(),
            message: tx_abort.message().unpack(),
        })
    }
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxInitRBF {
    pub channel_id: Hash256,
    pub fee_rate: u64,
}

impl From<TxInitRBF> for molecule_pcn::TxInitRBF {
    fn from(tx_init_rbf: TxInitRBF) -> Self {
        molecule_pcn::TxInitRBF::new_builder()
            .channel_id(tx_init_rbf.channel_id.into())
            .fee_rate(tx_init_rbf.fee_rate.pack())
            .build()
    }
}

impl TryFrom<molecule_pcn::TxInitRBF> for TxInitRBF {
    type Error = Error;

    fn try_from(tx_init_rbf: molecule_pcn::TxInitRBF) -> Result<Self, Self::Error> {
        Ok(TxInitRBF {
            channel_id: tx_init_rbf.channel_id().into(),
            fee_rate: tx_init_rbf.fee_rate().unpack(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxAckRBF {
    pub channel_id: Hash256,
}

impl From<TxAckRBF> for molecule_pcn::TxAckRBF {
    fn from(tx_ack_rbf: TxAckRBF) -> Self {
        molecule_pcn::TxAckRBF::new_builder()
            .channel_id(tx_ack_rbf.channel_id.into())
            .build()
    }
}

impl TryFrom<molecule_pcn::TxAckRBF> for TxAckRBF {
    type Error = Error;

    fn try_from(tx_ack_rbf: molecule_pcn::TxAckRBF) -> Result<Self, Self::Error> {
        Ok(TxAckRBF {
            channel_id: tx_ack_rbf.channel_id().into(),
        })
    }
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shutdown {
    pub channel_id: Hash256,
    #[serde_as(as = "EntityWrapperBase64<Script>")]
    pub close_script: Script,
}

impl From<Shutdown> for molecule_pcn::Shutdown {
    fn from(shutdown: Shutdown) -> Self {
        molecule_pcn::Shutdown::new_builder()
            .channel_id(shutdown.channel_id.into())
            .close_script(shutdown.close_script.into())
            .build()
    }
}

impl TryFrom<molecule_pcn::Shutdown> for Shutdown {
    type Error = Error;

    fn try_from(shutdown: molecule_pcn::Shutdown) -> Result<Self, Self::Error> {
        Ok(Shutdown {
            channel_id: shutdown.channel_id().into(),
            close_script: shutdown.close_script(),
        })
    }
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClosingSigned {
    pub channel_id: Hash256,
    pub fee: u64,
    pub signature: Signature,
}

impl From<ClosingSigned> for molecule_pcn::ClosingSigned {
    fn from(closing_signed: ClosingSigned) -> Self {
        molecule_pcn::ClosingSigned::new_builder()
            .channel_id(closing_signed.channel_id.into())
            .fee(closing_signed.fee.pack())
            .signature(closing_signed.signature.into())
            .build()
    }
}

impl TryFrom<molecule_pcn::ClosingSigned> for ClosingSigned {
    type Error = Error;

    fn try_from(closing_signed: molecule_pcn::ClosingSigned) -> Result<Self, Self::Error> {
        Ok(ClosingSigned {
            channel_id: closing_signed.channel_id().into(),
            fee: closing_signed.fee().unpack(),
            signature: closing_signed.signature().try_into()?,
        })
    }
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddTlc {
    pub channel_id: Hash256,
    pub tlc_id: u64,
    pub amount: u64,
    pub payment_hash: Hash256,
    pub expiry: u64,
}

impl From<AddTlc> for molecule_pcn::AddTlc {
    fn from(add_tlc: AddTlc) -> Self {
        molecule_pcn::AddTlc::new_builder()
            .channel_id(add_tlc.channel_id.into())
            .tlc_id(add_tlc.tlc_id.pack())
            .amount(add_tlc.amount.pack())
            .payment_hash(add_tlc.payment_hash.into())
            .expiry(add_tlc.expiry.pack())
            .build()
    }
}

impl TryFrom<molecule_pcn::AddTlc> for AddTlc {
    type Error = Error;

    fn try_from(add_tlc: molecule_pcn::AddTlc) -> Result<Self, Self::Error> {
        Ok(AddTlc {
            channel_id: add_tlc.channel_id().into(),
            tlc_id: add_tlc.tlc_id().unpack(),
            amount: add_tlc.amount().unpack(),
            payment_hash: add_tlc.payment_hash().into(),
            expiry: add_tlc.expiry().unpack(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlcsSigned {
    pub channel_id: Hash256,
    pub signature: Signature,
    pub tlc_signatures: Vec<Signature>,
}

impl From<TlcsSigned> for molecule_pcn::TlcsSigned {
    fn from(tlcs_signed: TlcsSigned) -> Self {
        molecule_pcn::TlcsSigned::new_builder()
            .channel_id(tlcs_signed.channel_id.into())
            .signature(tlcs_signed.signature.into())
            .tlc_signatures(
                SignatureVec::new_builder()
                    .set(
                        tlcs_signed
                            .tlc_signatures
                            .into_iter()
                            .map(|tlc_signature| tlc_signature.into())
                            .collect(),
                    )
                    .build(),
            )
            .build()
    }
}

impl TryFrom<molecule_pcn::TlcsSigned> for TlcsSigned {
    type Error = Error;

    fn try_from(tlcs_signed: molecule_pcn::TlcsSigned) -> Result<Self, Self::Error> {
        Ok(TlcsSigned {
            channel_id: tlcs_signed.channel_id().into(),
            signature: tlcs_signed.signature().try_into()?,
            tlc_signatures: tlcs_signed
                .tlc_signatures()
                .into_iter()
                .map(|tlc_signature| tlc_signature.try_into())
                .collect::<Result<Vec<Signature>, Error>>()?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeAndAck {
    pub channel_id: Hash256,
    pub per_commitment_secret: Hash256,
    pub next_per_commitment_point: Pubkey,
}

impl From<RevokeAndAck> for molecule_pcn::RevokeAndAck {
    fn from(revoke_and_ack: RevokeAndAck) -> Self {
        molecule_pcn::RevokeAndAck::new_builder()
            .channel_id(revoke_and_ack.channel_id.into())
            .per_commitment_secret(revoke_and_ack.per_commitment_secret.into())
            .next_per_commitment_point(revoke_and_ack.next_per_commitment_point.into())
            .build()
    }
}

impl TryFrom<molecule_pcn::RevokeAndAck> for RevokeAndAck {
    type Error = Error;

    fn try_from(revoke_and_ack: molecule_pcn::RevokeAndAck) -> Result<Self, Self::Error> {
        Ok(RevokeAndAck {
            channel_id: revoke_and_ack.channel_id().into(),
            per_commitment_secret: revoke_and_ack.per_commitment_secret().into(),
            next_per_commitment_point: revoke_and_ack.next_per_commitment_point().try_into()?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveTlcFulfill {
    pub payment_preimage: Hash256,
}

impl From<RemoveTlcFulfill> for molecule_pcn::RemoveTlcFulfill {
    fn from(remove_tlc_fulfill: RemoveTlcFulfill) -> Self {
        molecule_pcn::RemoveTlcFulfill::new_builder()
            .payment_preimage(remove_tlc_fulfill.payment_preimage.into())
            .build()
    }
}

impl TryFrom<molecule_pcn::RemoveTlcFulfill> for RemoveTlcFulfill {
    type Error = Error;

    fn try_from(remove_tlc_fulfill: molecule_pcn::RemoveTlcFulfill) -> Result<Self, Self::Error> {
        Ok(RemoveTlcFulfill {
            payment_preimage: remove_tlc_fulfill.payment_preimage().into(),
        })
    }
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveTlcFail {
    pub error_code: u32,
}

impl From<RemoveTlcFail> for molecule_pcn::RemoveTlcFail {
    fn from(remove_tlc_fail: RemoveTlcFail) -> Self {
        molecule_pcn::RemoveTlcFail::new_builder()
            .error_code(remove_tlc_fail.error_code.pack())
            .build()
    }
}

impl TryFrom<molecule_pcn::RemoveTlcFail> for RemoveTlcFail {
    type Error = Error;

    fn try_from(remove_tlc_fail: molecule_pcn::RemoveTlcFail) -> Result<Self, Self::Error> {
        Ok(RemoveTlcFail {
            error_code: remove_tlc_fail.error_code().unpack(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RemoveTlcReason {
    RemoveTlcFulfill(RemoveTlcFulfill),
    RemoveTlcFail(RemoveTlcFail),
}

impl From<RemoveTlcReason> for molecule_pcn::RemoveTlcReasonUnion {
    fn from(remove_tlc_reason: RemoveTlcReason) -> Self {
        match remove_tlc_reason {
            RemoveTlcReason::RemoveTlcFulfill(remove_tlc_fulfill) => {
                molecule_pcn::RemoveTlcReasonUnion::RemoveTlcFulfill(remove_tlc_fulfill.into())
            }
            RemoveTlcReason::RemoveTlcFail(remove_tlc_fail) => {
                molecule_pcn::RemoveTlcReasonUnion::RemoveTlcFail(remove_tlc_fail.into())
            }
        }
    }
}

impl From<RemoveTlcReason> for molecule_pcn::RemoveTlcReason {
    fn from(remove_tlc_reason: RemoveTlcReason) -> Self {
        molecule_pcn::RemoveTlcReason::new_builder()
            .set(remove_tlc_reason)
            .build()
    }
}

impl TryFrom<molecule_pcn::RemoveTlcReason> for RemoveTlcReason {
    type Error = Error;

    fn try_from(remove_tlc_reason: molecule_pcn::RemoveTlcReason) -> Result<Self, Self::Error> {
        match remove_tlc_reason.to_enum() {
            molecule_pcn::RemoveTlcReasonUnion::RemoveTlcFulfill(remove_tlc_fulfill) => Ok(
                RemoveTlcReason::RemoveTlcFulfill(remove_tlc_fulfill.try_into()?),
            ),
            molecule_pcn::RemoveTlcReasonUnion::RemoveTlcFail(remove_tlc_fail) => {
                Ok(RemoveTlcReason::RemoveTlcFail(remove_tlc_fail.try_into()?))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveTlc {
    pub channel_id: Hash256,
    pub tlc_id: u64,
    pub reason: RemoveTlcReason,
}

impl From<RemoveTlc> for molecule_pcn::RemoveTlc {
    fn from(remove_tlc: RemoveTlc) -> Self {
        molecule_pcn::RemoveTlc::new_builder()
            .channel_id(remove_tlc.channel_id.into())
            .tlc_id(remove_tlc.tlc_id.pack())
            .reason(
                molecule_pcn::RemoveTlcReason::new_builder()
                    .set(remove_tlc.reason)
                    .build(),
            )
            .build()
    }
}

impl TryFrom<molecule_pcn::RemoveTlc> for RemoveTlc {
    type Error = Error;

    fn try_from(remove_tlc: molecule_pcn::RemoveTlc) -> Result<Self, Self::Error> {
        Ok(RemoveTlc {
            channel_id: remove_tlc.channel_id().into(),
            tlc_id: remove_tlc.tlc_id().unpack(),
            reason: remove_tlc.reason().try_into()?,
        })
    }
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestMessage {
    #[serde_as(as = "Base64")]
    pub bytes: Vec<u8>,
}

impl From<TestMessage> for molecule_pcn::TestMessage {
    fn from(test_message: TestMessage) -> Self {
        molecule_pcn::TestMessage::new_builder()
            .bytes(test_message.bytes.pack())
            .build()
    }
}

impl TryFrom<molecule_pcn::TestMessage> for TestMessage {
    type Error = Error;

    fn try_from(test_message: molecule_pcn::TestMessage) -> Result<Self, Self::Error> {
        Ok(TestMessage {
            bytes: test_message.bytes().unpack(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PCNMessage {
    TestMessage(TestMessage),
    OpenChannel(OpenChannel),
    AcceptChannel(AcceptChannel),
    CommitmentSigned(CommitmentSigned),
    TxSignatures(TxSignatures),
    ChannelReady(ChannelReady),
    TxAdd(TxAdd),
    TxRemove(TxRemove),
    TxComplete(TxComplete),
    TxAbort(TxAbort),
    TxInitRBF(TxInitRBF),
    TxAckRBF(TxAckRBF),
    Shutdown(Shutdown),
    ClosingSigned(ClosingSigned),
    AddTlc(AddTlc),
    TlcsSigned(TlcsSigned),
    RevokeAndAck(RevokeAndAck),
    RemoveTlc(RemoveTlc),
}

impl From<PCNMessage> for molecule_pcn::PCNMessageUnion {
    fn from(pcn_message: PCNMessage) -> Self {
        match pcn_message {
            PCNMessage::TestMessage(test_message) => {
                molecule_pcn::PCNMessageUnion::TestMessage(test_message.into())
            }
            PCNMessage::OpenChannel(open_channel) => {
                molecule_pcn::PCNMessageUnion::OpenChannel(open_channel.into())
            }
            PCNMessage::AcceptChannel(accept_channel) => {
                molecule_pcn::PCNMessageUnion::AcceptChannel(accept_channel.into())
            }
            PCNMessage::CommitmentSigned(commitment_signed) => {
                molecule_pcn::PCNMessageUnion::CommitmentSigned(commitment_signed.into())
            }
            PCNMessage::TxSignatures(tx_signatures) => {
                molecule_pcn::PCNMessageUnion::TxSignatures(tx_signatures.into())
            }
            PCNMessage::ChannelReady(channel_ready) => {
                molecule_pcn::PCNMessageUnion::ChannelReady(channel_ready.into())
            }
            PCNMessage::TxAdd(tx_add) => molecule_pcn::PCNMessageUnion::TxAdd(tx_add.into()),
            PCNMessage::TxRemove(tx_remove) => {
                molecule_pcn::PCNMessageUnion::TxRemove(tx_remove.into())
            }
            PCNMessage::TxComplete(tx_complete) => {
                molecule_pcn::PCNMessageUnion::TxComplete(tx_complete.into())
            }
            PCNMessage::TxAbort(tx_abort) => {
                molecule_pcn::PCNMessageUnion::TxAbort(tx_abort.into())
            }
            PCNMessage::TxInitRBF(tx_init_rbf) => {
                molecule_pcn::PCNMessageUnion::TxInitRBF(tx_init_rbf.into())
            }
            PCNMessage::TxAckRBF(tx_ack_rbf) => {
                molecule_pcn::PCNMessageUnion::TxAckRBF(tx_ack_rbf.into())
            }
            PCNMessage::Shutdown(shutdown) => {
                molecule_pcn::PCNMessageUnion::Shutdown(shutdown.into())
            }
            PCNMessage::ClosingSigned(closing_signed) => {
                molecule_pcn::PCNMessageUnion::ClosingSigned(closing_signed.into())
            }
            PCNMessage::AddTlc(add_tlc) => molecule_pcn::PCNMessageUnion::AddTlc(add_tlc.into()),
            PCNMessage::RemoveTlc(remove_tlc) => {
                molecule_pcn::PCNMessageUnion::RemoveTlc(remove_tlc.into())
            }
            PCNMessage::RevokeAndAck(revoke_and_ack) => {
                molecule_pcn::PCNMessageUnion::RevokeAndAck(revoke_and_ack.into())
            }
            PCNMessage::TlcsSigned(tlcs_signed) => {
                molecule_pcn::PCNMessageUnion::TlcsSigned(tlcs_signed.into())
            }
        }
    }
}

impl From<PCNMessage> for molecule_pcn::PCNMessage {
    fn from(pcn_message: PCNMessage) -> Self {
        molecule_pcn::PCNMessage::new_builder()
            .set(pcn_message)
            .build()
    }
}

impl TryFrom<molecule_pcn::PCNMessage> for PCNMessage {
    type Error = Error;

    fn try_from(pcn_message: molecule_pcn::PCNMessage) -> Result<Self, Self::Error> {
        Ok(match pcn_message.to_enum() {
            molecule_pcn::PCNMessageUnion::TestMessage(test_message) => {
                PCNMessage::TestMessage(test_message.try_into()?)
            }
            molecule_pcn::PCNMessageUnion::OpenChannel(open_channel) => {
                PCNMessage::OpenChannel(open_channel.try_into()?)
            }
            molecule_pcn::PCNMessageUnion::AcceptChannel(accept_channel) => {
                PCNMessage::AcceptChannel(accept_channel.try_into()?)
            }
            molecule_pcn::PCNMessageUnion::CommitmentSigned(commitment_signed) => {
                PCNMessage::CommitmentSigned(commitment_signed.try_into()?)
            }
            molecule_pcn::PCNMessageUnion::TxSignatures(tx_signatures) => {
                PCNMessage::TxSignatures(tx_signatures.try_into()?)
            }
            molecule_pcn::PCNMessageUnion::ChannelReady(channel_ready) => {
                PCNMessage::ChannelReady(channel_ready.try_into()?)
            }
            molecule_pcn::PCNMessageUnion::TxAdd(tx_add) => PCNMessage::TxAdd(tx_add.try_into()?),
            molecule_pcn::PCNMessageUnion::TxRemove(tx_remove) => {
                PCNMessage::TxRemove(tx_remove.try_into()?)
            }
            molecule_pcn::PCNMessageUnion::TxComplete(tx_complete) => {
                PCNMessage::TxComplete(tx_complete.try_into()?)
            }
            molecule_pcn::PCNMessageUnion::TxAbort(tx_abort) => {
                PCNMessage::TxAbort(tx_abort.try_into()?)
            }
            molecule_pcn::PCNMessageUnion::TxInitRBF(tx_init_rbf) => {
                PCNMessage::TxInitRBF(tx_init_rbf.try_into()?)
            }
            molecule_pcn::PCNMessageUnion::TxAckRBF(tx_ack_rbf) => {
                PCNMessage::TxAckRBF(tx_ack_rbf.try_into()?)
            }
            molecule_pcn::PCNMessageUnion::Shutdown(shutdown) => {
                PCNMessage::Shutdown(shutdown.try_into()?)
            }
            molecule_pcn::PCNMessageUnion::ClosingSigned(closing_signed) => {
                PCNMessage::ClosingSigned(closing_signed.try_into()?)
            }
            molecule_pcn::PCNMessageUnion::AddTlc(add_tlc) => {
                PCNMessage::AddTlc(add_tlc.try_into()?)
            }
            molecule_pcn::PCNMessageUnion::RemoveTlc(remove_tlc) => {
                PCNMessage::RemoveTlc(remove_tlc.try_into()?)
            }
            molecule_pcn::PCNMessageUnion::TlcsSigned(tlcs_signed) => {
                PCNMessage::TlcsSigned(tlcs_signed.try_into()?)
            }
            molecule_pcn::PCNMessageUnion::RevokeAndAck(revoke_and_ack) => {
                PCNMessage::RevokeAndAck(revoke_and_ack.try_into()?)
            }
        })
    }
}

macro_rules! impl_traits {
    ($t:ident) => {
        impl $t {
            pub fn to_molecule_bytes(&self) -> molecule::bytes::Bytes {
                // TODO: we cloned twice here, both in self.clone and as_bytes.
                molecule_pcn::$t::from(self.clone()).as_bytes()
            }
        }

        impl $t {
            pub fn from_molecule_slice(data: &[u8]) -> Result<Self, Error> {
                molecule_pcn::$t::from_slice(data)
                    .map_err(Into::into)
                    .and_then(TryInto::try_into)
            }
        }
    };
}

impl_traits!(TestMessage);
impl_traits!(OpenChannel);
impl_traits!(AcceptChannel);
impl_traits!(CommitmentSigned);
impl_traits!(TxSignatures);
impl_traits!(ChannelReady);
impl_traits!(TxAdd);
impl_traits!(TxRemove);
impl_traits!(TxComplete);
impl_traits!(TxAbort);
impl_traits!(TxInitRBF);
impl_traits!(TxAckRBF);
impl_traits!(Shutdown);
impl_traits!(ClosingSigned);
impl_traits!(AddTlc);
impl_traits!(TlcsSigned);
impl_traits!(RevokeAndAck);
impl_traits!(RemoveTlc);
impl_traits!(PCNMessage);

#[cfg(test)]
mod tests {
    use super::{secp256k1_instance, Pubkey};

    use secp256k1::SecretKey;

    #[test]
    fn test_serde_public_key() {
        let sk = SecretKey::from_slice(&[42; 32]).unwrap();
        let public_key = Pubkey::from(sk.public_key(secp256k1_instance()));
        let pk_str = serde_json::to_string(&public_key).unwrap();
        assert_eq!(
            "\"035be5e9478209674a96e60f1f037f6176540fd001fa1d64694770c56a7709c42c\"",
            &pk_str
        );
        let pubkey: Pubkey = serde_json::from_str(&pk_str).unwrap();
        assert_eq!(pubkey, Pubkey::from(public_key))
    }
}
