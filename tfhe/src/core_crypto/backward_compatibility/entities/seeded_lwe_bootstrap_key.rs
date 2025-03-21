use tfhe_versionable::deprecation::{Deprecable, Deprecated};
use tfhe_versionable::VersionsDispatch;

use crate::core_crypto::prelude::{Container, SeededLweBootstrapKey, UnsignedInteger};

impl<C: Container> Deprecable for SeededLweBootstrapKey<C>
where
    C::Element: UnsignedInteger,
{
    const TYPE_NAME: &'static str = "SeededLweBootstrapKey";
    const MIN_SUPPORTED_APP_VERSION: &'static str = "TFHE-rs v0.10";
}

#[derive(VersionsDispatch)]
pub enum SeededLweBootstrapKeyVersions<C: Container>
where
    C::Element: UnsignedInteger,
{
    V0(Deprecated<SeededLweBootstrapKey<C>>),
    V1(SeededLweBootstrapKey<C>),
}
