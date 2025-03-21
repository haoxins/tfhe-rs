//! Module containing primitives pertaining to the generation of
//! [`standard LWE bootstrap keys`](`LweBootstrapKey`) and [`seeded standard LWE bootstrap
//! keys`](`SeededLweBootstrapKey`).

use crate::core_crypto::algorithms::*;
use crate::core_crypto::commons::generators::EncryptionRandomGenerator;
use crate::core_crypto::commons::math::random::{DefaultRandomGenerator, Distribution, Uniform};
use crate::core_crypto::commons::parameters::*;
use crate::core_crypto::commons::traits::*;
use crate::core_crypto::entities::*;
use rayon::prelude::*;

/// Fill an [`LWE bootstrap key`](`LweBootstrapKey`) with an actual bootstrapping key constructed
/// from an input key [`LWE secret key`](`LweSecretKey`) and an output key
/// [`GLWE secret key`](`GlweSecretKey`)
///
/// Consider using [`par_generate_lwe_bootstrap_key`] for better key generation times.
///
/// ```rust
/// use tfhe::core_crypto::prelude::*;
///
/// // DISCLAIMER: these toy example parameters are not guaranteed to be secure or yield correct
/// // computations
/// // Define parameters for LweBootstrapKey creation
/// let input_lwe_dimension = LweDimension(742);
/// let decomp_base_log = DecompositionBaseLog(3);
/// let decomp_level_count = DecompositionLevelCount(5);
/// let glwe_dimension = GlweDimension(1);
/// let polynomial_size = PolynomialSize(1024);
/// let glwe_noise_distribution =
///     Gaussian::from_dispersion_parameter(StandardDev(0.00000000000000029403601535432533), 0.0);
/// let ciphertext_modulus = CiphertextModulus::new_native();
///
/// // Create the PRNG
/// let mut seeder = new_seeder();
/// let seeder = seeder.as_mut();
/// let mut encryption_generator =
///     EncryptionRandomGenerator::<DefaultRandomGenerator>::new(seeder.seed(), seeder);
/// let mut secret_generator = SecretRandomGenerator::<DefaultRandomGenerator>::new(seeder.seed());
///
/// // Create the LweSecretKey
/// let input_lwe_secret_key =
///     allocate_and_generate_new_binary_lwe_secret_key(input_lwe_dimension, &mut secret_generator);
/// let output_glwe_secret_key = allocate_and_generate_new_binary_glwe_secret_key(
///     glwe_dimension,
///     polynomial_size,
///     &mut secret_generator,
/// );
///
/// let mut bsk = LweBootstrapKey::new(
///     0u64,
///     glwe_dimension.to_glwe_size(),
///     polynomial_size,
///     decomp_base_log,
///     decomp_level_count,
///     input_lwe_dimension,
///     ciphertext_modulus,
/// );
///
/// generate_lwe_bootstrap_key(
///     &input_lwe_secret_key,
///     &output_glwe_secret_key,
///     &mut bsk,
///     glwe_noise_distribution,
///     &mut encryption_generator,
/// );
///
/// for (ggsw, &input_key_bit) in bsk.iter().zip(input_lwe_secret_key.as_ref()) {
///     let decrypted_ggsw = decrypt_constant_ggsw_ciphertext(&output_glwe_secret_key, &ggsw);
///     assert_eq!(decrypted_ggsw.0, input_key_bit)
/// }
/// ```
pub fn generate_lwe_bootstrap_key<
    InputScalar,
    OutputScalar,
    NoiseDistribution,
    InputKeyCont,
    OutputKeyCont,
    OutputCont,
    Gen,
>(
    input_lwe_secret_key: &LweSecretKey<InputKeyCont>,
    output_glwe_secret_key: &GlweSecretKey<OutputKeyCont>,
    output: &mut LweBootstrapKey<OutputCont>,
    noise_distribution: NoiseDistribution,
    generator: &mut EncryptionRandomGenerator<Gen>,
) where
    InputScalar: Copy + CastInto<OutputScalar>,
    OutputScalar: Encryptable<Uniform, NoiseDistribution>,
    NoiseDistribution: Distribution,
    InputKeyCont: Container<Element = InputScalar>,
    OutputKeyCont: Container<Element = OutputScalar>,
    OutputCont: ContainerMut<Element = OutputScalar>,
    Gen: ByteRandomGenerator,
{
    assert!(
        output.input_lwe_dimension() == input_lwe_secret_key.lwe_dimension(),
        "Mismatched LweDimension between input LWE secret key and LWE bootstrap key. \
        Input LWE secret key LweDimension: {:?}, LWE bootstrap key input LweDimension {:?}.",
        input_lwe_secret_key.lwe_dimension(),
        output.input_lwe_dimension()
    );

    assert!(
        output.glwe_size() == output_glwe_secret_key.glwe_dimension().to_glwe_size(),
        "Mismatched GlweSize between output GLWE secret key and LWE bootstrap key. \
        Output GLWE secret key GlweSize: {:?}, LWE bootstrap key GlweSize {:?}.",
        output_glwe_secret_key.glwe_dimension().to_glwe_size(),
        output.glwe_size()
    );

    assert!(
        output.polynomial_size() == output_glwe_secret_key.polynomial_size(),
        "Mismatched PolynomialSize between output GLWE secret key and LWE bootstrap key. \
        Output GLWE secret key PolynomialSize: {:?}, LWE bootstrap key PolynomialSize {:?}.",
        output_glwe_secret_key.polynomial_size(),
        output.polynomial_size()
    );

    let gen_iter = generator
        .try_fork_from_config(output.encryption_fork_config(Uniform, noise_distribution))
        .unwrap();

    for ((mut ggsw, &input_key_element), mut generator) in output
        .iter_mut()
        .zip(input_lwe_secret_key.as_ref())
        .zip(gen_iter)
    {
        encrypt_constant_ggsw_ciphertext(
            output_glwe_secret_key,
            &mut ggsw,
            Cleartext(input_key_element.cast_into()),
            noise_distribution,
            &mut generator,
        );
    }
}

/// Allocate a new [`LWE bootstrap key`](`LweBootstrapKey`) and fill it with an actual bootstrapping
/// key constructed from an input key [`LWE secret key`](`LweSecretKey`) and an output key
/// [`GLWE secret key`](`GlweSecretKey`).
///
/// Consider using [`par_allocate_and_generate_new_lwe_bootstrap_key`] for better key generation
/// times.
pub fn allocate_and_generate_new_lwe_bootstrap_key<
    InputScalar,
    OutputScalar,
    NoiseDistribution,
    InputKeyCont,
    OutputKeyCont,
    Gen,
>(
    input_lwe_secret_key: &LweSecretKey<InputKeyCont>,
    output_glwe_secret_key: &GlweSecretKey<OutputKeyCont>,
    decomp_base_log: DecompositionBaseLog,
    decomp_level_count: DecompositionLevelCount,
    noise_distribution: NoiseDistribution,
    ciphertext_modulus: CiphertextModulus<OutputScalar>,
    generator: &mut EncryptionRandomGenerator<Gen>,
) -> LweBootstrapKeyOwned<OutputScalar>
where
    InputScalar: Copy + CastInto<OutputScalar>,
    OutputScalar: Encryptable<Uniform, NoiseDistribution>,
    NoiseDistribution: Distribution,
    InputKeyCont: Container<Element = InputScalar>,
    OutputKeyCont: Container<Element = OutputScalar>,
    Gen: ByteRandomGenerator,
{
    let mut bsk = LweBootstrapKeyOwned::new(
        OutputScalar::ZERO,
        output_glwe_secret_key.glwe_dimension().to_glwe_size(),
        output_glwe_secret_key.polynomial_size(),
        decomp_base_log,
        decomp_level_count,
        input_lwe_secret_key.lwe_dimension(),
        ciphertext_modulus,
    );

    generate_lwe_bootstrap_key(
        input_lwe_secret_key,
        output_glwe_secret_key,
        &mut bsk,
        noise_distribution,
        generator,
    );

    bsk
}

/// Parallel variant of [`generate_lwe_bootstrap_key`], it is recommended to use this function for
/// better key generation times as LWE bootstrapping keys can be quite large.
///
/// # Example
///
/// ```rust
/// use tfhe::core_crypto::prelude::*;
///
/// // DISCLAIMER: these toy example parameters are not guaranteed to be secure or yield correct
/// // computations
/// // Define parameters for LweBootstrapKey creation
/// let input_lwe_dimension = LweDimension(742);
/// let decomp_base_log = DecompositionBaseLog(3);
/// let decomp_level_count = DecompositionLevelCount(5);
/// let glwe_dimension = GlweDimension(1);
/// let polynomial_size = PolynomialSize(1024);
/// let glwe_noise_distribution =
///     Gaussian::from_dispersion_parameter(StandardDev(0.00000000000000029403601535432533), 0.0);
/// let ciphertext_modulus = CiphertextModulus::new_native();
///
/// // Create the PRNG
/// let mut seeder = new_seeder();
/// let seeder = seeder.as_mut();
/// let mut encryption_generator =
///     EncryptionRandomGenerator::<DefaultRandomGenerator>::new(seeder.seed(), seeder);
/// let mut secret_generator = SecretRandomGenerator::<DefaultRandomGenerator>::new(seeder.seed());
///
/// // Create the LweSecretKey
/// let input_lwe_secret_key = allocate_and_generate_new_binary_lwe_secret_key::<u64, _>(
///     input_lwe_dimension,
///     &mut secret_generator,
/// );
/// let output_glwe_secret_key = allocate_and_generate_new_binary_glwe_secret_key(
///     glwe_dimension,
///     polynomial_size,
///     &mut secret_generator,
/// );
///
/// let mut bsk = LweBootstrapKey::new(
///     0u64,
///     glwe_dimension.to_glwe_size(),
///     polynomial_size,
///     decomp_base_log,
///     decomp_level_count,
///     input_lwe_dimension,
///     ciphertext_modulus,
/// );
///
/// par_generate_lwe_bootstrap_key(
///     &input_lwe_secret_key,
///     &output_glwe_secret_key,
///     &mut bsk,
///     glwe_noise_distribution,
///     &mut encryption_generator,
/// );
///
/// assert!(!bsk.as_ref().iter().all(|&x| x == 0));
/// ```
pub fn par_generate_lwe_bootstrap_key<
    InputScalar,
    OutputScalar,
    NoiseDistribution,
    InputKeyCont,
    OutputKeyCont,
    OutputCont,
    Gen,
>(
    input_lwe_secret_key: &LweSecretKey<InputKeyCont>,
    output_glwe_secret_key: &GlweSecretKey<OutputKeyCont>,
    output: &mut LweBootstrapKey<OutputCont>,
    noise_distribution: NoiseDistribution,
    generator: &mut EncryptionRandomGenerator<Gen>,
) where
    InputScalar: Copy + CastInto<OutputScalar> + Sync,
    OutputScalar: Encryptable<Uniform, NoiseDistribution> + Sync + Send,
    NoiseDistribution: Distribution + Sync,
    InputKeyCont: Container<Element = InputScalar>,
    OutputKeyCont: Container<Element = OutputScalar> + Sync,
    OutputCont: ContainerMut<Element = OutputScalar>,
    Gen: ParallelByteRandomGenerator,
{
    assert!(
        output.input_lwe_dimension() == input_lwe_secret_key.lwe_dimension(),
        "Mismatched LweDimension between input LWE secret key and LWE bootstrap key. \
        Input LWE secret key LweDimension: {:?}, LWE bootstrap key input LweDimension {:?}.",
        input_lwe_secret_key.lwe_dimension(),
        output.input_lwe_dimension()
    );

    assert!(
        output.glwe_size() == output_glwe_secret_key.glwe_dimension().to_glwe_size(),
        "Mismatched GlweSize between output GLWE secret key and LWE bootstrap key. \
        Output GLWE secret key GlweSize: {:?}, LWE bootstrap key GlweSize {:?}.",
        output_glwe_secret_key.glwe_dimension().to_glwe_size(),
        output.glwe_size()
    );

    assert!(
        output.polynomial_size() == output_glwe_secret_key.polynomial_size(),
        "Mismatched PolynomialSize between output GLWE secret key and LWE bootstrap key. \
        Output GLWE secret key PolynomialSize: {:?}, LWE bootstrap key PolynomialSize {:?}.",
        output_glwe_secret_key.polynomial_size(),
        output.polynomial_size()
    );

    let gen_iter = generator
        .par_try_fork_from_config(output.encryption_fork_config(Uniform, noise_distribution))
        .unwrap();

    output
        .par_iter_mut()
        .zip(input_lwe_secret_key.as_ref().par_iter())
        .zip(gen_iter)
        .for_each(|((mut ggsw, &input_key_element), mut generator)| {
            par_encrypt_constant_ggsw_ciphertext(
                output_glwe_secret_key,
                &mut ggsw,
                Cleartext(input_key_element.cast_into()),
                noise_distribution,
                &mut generator,
            );
        });
}

/// Parallel variant of [`allocate_and_generate_new_lwe_bootstrap_key`], it is recommended to use
/// this function for better key generation times as LWE bootstrapping keys can be quite large.
///
/// See [`programmable_bootstrap_lwe_ciphertext`] for usage.
pub fn par_allocate_and_generate_new_lwe_bootstrap_key<
    InputScalar,
    OutputScalar,
    NoiseDistribution,
    InputKeyCont,
    OutputKeyCont,
    Gen,
>(
    input_lwe_secret_key: &LweSecretKey<InputKeyCont>,
    output_glwe_secret_key: &GlweSecretKey<OutputKeyCont>,
    decomp_base_log: DecompositionBaseLog,
    decomp_level_count: DecompositionLevelCount,
    noise_distribution: NoiseDistribution,
    ciphertext_modulus: CiphertextModulus<OutputScalar>,
    generator: &mut EncryptionRandomGenerator<Gen>,
) -> LweBootstrapKeyOwned<OutputScalar>
where
    InputScalar: Copy + CastInto<OutputScalar> + Sync,
    OutputScalar: Encryptable<Uniform, NoiseDistribution> + Sync + Send,
    NoiseDistribution: Distribution + Sync,
    InputKeyCont: Container<Element = InputScalar>,
    OutputKeyCont: Container<Element = OutputScalar> + Sync,
    Gen: ParallelByteRandomGenerator,
{
    let mut bsk = LweBootstrapKeyOwned::new(
        OutputScalar::ZERO,
        output_glwe_secret_key.glwe_dimension().to_glwe_size(),
        output_glwe_secret_key.polynomial_size(),
        decomp_base_log,
        decomp_level_count,
        input_lwe_secret_key.lwe_dimension(),
        ciphertext_modulus,
    );

    par_generate_lwe_bootstrap_key(
        input_lwe_secret_key,
        output_glwe_secret_key,
        &mut bsk,
        noise_distribution,
        generator,
    );

    bsk
}

/// Fill a [`seeded LWE bootstrap key`](`SeededLweBootstrapKey`) with an actual seeded bootstrapping
/// key constructed from an input key [`LWE secret key`](`LweSecretKey`) and an output key
/// [`GLWE secret key`](`GlweSecretKey`)
///
/// Consider using [`par_generate_seeded_lwe_bootstrap_key`] for better key generation times.
pub fn generate_seeded_lwe_bootstrap_key<
    InputScalar,
    OutputScalar,
    NoiseDistribution,
    InputKeyCont,
    OutputKeyCont,
    OutputCont,
    NoiseSeeder,
>(
    input_lwe_secret_key: &LweSecretKey<InputKeyCont>,
    output_glwe_secret_key: &GlweSecretKey<OutputKeyCont>,
    output: &mut SeededLweBootstrapKey<OutputCont>,
    noise_distribution: NoiseDistribution,
    noise_seeder: &mut NoiseSeeder,
) where
    InputScalar: Copy + CastInto<OutputScalar>,
    OutputScalar: Encryptable<Uniform, NoiseDistribution>,
    NoiseDistribution: Distribution,
    InputKeyCont: Container<Element = InputScalar>,
    OutputKeyCont: Container<Element = OutputScalar>,
    OutputCont: ContainerMut<Element = OutputScalar>,
    // Maybe Sized allows to pass Box<dyn Seeder>.
    NoiseSeeder: Seeder + ?Sized,
{
    assert!(
        output.input_lwe_dimension() == input_lwe_secret_key.lwe_dimension(),
        "Mismatched LweDimension between input LWE secret key and LWE bootstrap key. \
        Input LWE secret key LweDimension: {:?}, LWE bootstrap key input LweDimension {:?}.",
        input_lwe_secret_key.lwe_dimension(),
        output.input_lwe_dimension()
    );

    assert!(
        output.glwe_size() == output_glwe_secret_key.glwe_dimension().to_glwe_size(),
        "Mismatched GlweSize between output GLWE secret key and LWE bootstrap key. \
        Output GLWE secret key GlweSize: {:?}, LWE bootstrap key GlweSize {:?}.",
        output_glwe_secret_key.glwe_dimension().to_glwe_size(),
        output.glwe_size()
    );

    assert!(
        output.polynomial_size() == output_glwe_secret_key.polynomial_size(),
        "Mismatched PolynomialSize between output GLWE secret key and LWE bootstrap key. \
        Output GLWE secret key PolynomialSize: {:?}, LWE bootstrap key PolynomialSize {:?}.",
        output_glwe_secret_key.polynomial_size(),
        output.polynomial_size()
    );

    let mut generator = EncryptionRandomGenerator::<DefaultRandomGenerator>::new(
        output.compression_seed().seed,
        noise_seeder,
    );

    let gen_iter = generator
        .try_fork_from_config(output.encryption_fork_config(Uniform, noise_distribution))
        .unwrap();

    for ((mut ggsw, &input_key_element), mut generator) in output
        .iter_mut()
        .zip(input_lwe_secret_key.as_ref())
        .zip(gen_iter)
    {
        encrypt_constant_seeded_ggsw_ciphertext_with_pre_seeded_generator(
            output_glwe_secret_key,
            &mut ggsw,
            Cleartext(input_key_element.cast_into()),
            noise_distribution,
            &mut generator,
        );
    }
}

/// Allocate a new [`seeded LWE bootstrap key`](`SeededLweBootstrapKey`) and fill it with an actual
/// seeded bootstrapping key constructed from an input key [`LWE secret key`](`LweSecretKey`) and an
/// output key [`GLWE secret key`](`GlweSecretKey`)
///
/// Consider using [`par_allocate_and_generate_new_seeded_lwe_bootstrap_key`] for better key
/// generation times.
pub fn allocate_and_generate_new_seeded_lwe_bootstrap_key<
    InputScalar,
    OutputScalar,
    NoiseDistribution,
    InputKeyCont,
    OutputKeyCont,
    NoiseSeeder,
>(
    input_lwe_secret_key: &LweSecretKey<InputKeyCont>,
    output_glwe_secret_key: &GlweSecretKey<OutputKeyCont>,
    decomp_base_log: DecompositionBaseLog,
    decomp_level_count: DecompositionLevelCount,
    noise_distribution: NoiseDistribution,
    ciphertext_modulus: CiphertextModulus<OutputScalar>,
    noise_seeder: &mut NoiseSeeder,
) -> SeededLweBootstrapKeyOwned<OutputScalar>
where
    InputScalar: Copy + CastInto<OutputScalar>,
    OutputScalar: Encryptable<Uniform, NoiseDistribution>,
    NoiseDistribution: Distribution,
    InputKeyCont: Container<Element = InputScalar>,
    OutputKeyCont: Container<Element = OutputScalar>,
    // Maybe Sized allows to pass Box<dyn Seeder>.
    NoiseSeeder: Seeder + ?Sized,
{
    let mut bsk = SeededLweBootstrapKeyOwned::new(
        OutputScalar::ZERO,
        output_glwe_secret_key.glwe_dimension().to_glwe_size(),
        output_glwe_secret_key.polynomial_size(),
        decomp_base_log,
        decomp_level_count,
        input_lwe_secret_key.lwe_dimension(),
        noise_seeder.seed().into(),
        ciphertext_modulus,
    );

    generate_seeded_lwe_bootstrap_key(
        input_lwe_secret_key,
        output_glwe_secret_key,
        &mut bsk,
        noise_distribution,
        noise_seeder,
    );

    bsk
}

/// Parallel variant of [`generate_seeded_lwe_bootstrap_key`], it is recommended to use this
/// function for better key generation times as LWE bootstrapping keys can be quite large.
pub fn par_generate_seeded_lwe_bootstrap_key<
    InputScalar,
    OutputScalar,
    NoiseDistribution,
    InputKeyCont,
    OutputKeyCont,
    OutputCont,
    NoiseSeeder,
>(
    input_lwe_secret_key: &LweSecretKey<InputKeyCont>,
    output_glwe_secret_key: &GlweSecretKey<OutputKeyCont>,
    output: &mut SeededLweBootstrapKey<OutputCont>,
    noise_distribution: NoiseDistribution,
    noise_seeder: &mut NoiseSeeder,
) where
    InputScalar: Copy + CastInto<OutputScalar> + Sync,
    OutputScalar: Encryptable<Uniform, NoiseDistribution> + Sync + Send,
    NoiseDistribution: Distribution + Sync,
    InputKeyCont: Container<Element = InputScalar>,
    OutputKeyCont: Container<Element = OutputScalar> + Sync,
    OutputCont: ContainerMut<Element = OutputScalar>,
    // Maybe Sized allows to pass Box<dyn Seeder>.
    NoiseSeeder: Seeder + ?Sized,
{
    assert!(
        output.input_lwe_dimension() == input_lwe_secret_key.lwe_dimension(),
        "Mismatched LweDimension between input LWE secret key and LWE bootstrap key. \
        Input LWE secret key LweDimension: {:?}, LWE bootstrap key input LweDimension {:?}.",
        input_lwe_secret_key.lwe_dimension(),
        output.input_lwe_dimension()
    );

    assert!(
        output.glwe_size() == output_glwe_secret_key.glwe_dimension().to_glwe_size(),
        "Mismatched GlweSize between output GLWE secret key and LWE bootstrap key. \
        Output GLWE secret key GlweSize: {:?}, LWE bootstrap key GlweSize {:?}.",
        output_glwe_secret_key.glwe_dimension().to_glwe_size(),
        output.glwe_size()
    );

    assert!(
        output.polynomial_size() == output_glwe_secret_key.polynomial_size(),
        "Mismatched PolynomialSize between output GLWE secret key and LWE bootstrap key. \
        Output GLWE secret key PolynomialSize: {:?}, LWE bootstrap key PolynomialSize {:?}.",
        output_glwe_secret_key.polynomial_size(),
        output.polynomial_size()
    );

    let mut generator = EncryptionRandomGenerator::<DefaultRandomGenerator>::new(
        output.compression_seed().seed,
        noise_seeder,
    );

    let gen_iter = generator
        .par_try_fork_from_config(output.encryption_fork_config(Uniform, noise_distribution))
        .unwrap();

    output
        .par_iter_mut()
        .zip(input_lwe_secret_key.as_ref().par_iter())
        .zip(gen_iter)
        .for_each(|((mut ggsw, &input_key_element), mut generator)| {
            par_encrypt_constant_seeded_ggsw_ciphertext_with_pre_seeded_generator(
                output_glwe_secret_key,
                &mut ggsw,
                Cleartext(input_key_element.cast_into()),
                noise_distribution,
                &mut generator,
            );
        });
}

/// Parallel variant of [`allocate_and_generate_new_seeded_lwe_bootstrap_key`], it is recommended to
/// use this function for better key generation times as LWE bootstrapping keys can be quite large.
pub fn par_allocate_and_generate_new_seeded_lwe_bootstrap_key<
    InputScalar,
    OutputScalar,
    NoiseDistribution,
    InputKeyCont,
    OutputKeyCont,
    NoiseSeeder,
>(
    input_lwe_secret_key: &LweSecretKey<InputKeyCont>,
    output_glwe_secret_key: &GlweSecretKey<OutputKeyCont>,
    decomp_base_log: DecompositionBaseLog,
    decomp_level_count: DecompositionLevelCount,
    noise_distribution: NoiseDistribution,
    ciphertext_modulus: CiphertextModulus<OutputScalar>,
    noise_seeder: &mut NoiseSeeder,
) -> SeededLweBootstrapKeyOwned<OutputScalar>
where
    InputScalar: Copy + CastInto<OutputScalar> + Sync,
    OutputScalar: Encryptable<Uniform, NoiseDistribution> + Sync + Send,
    NoiseDistribution: Distribution + Sync,
    InputKeyCont: Container<Element = InputScalar>,
    OutputKeyCont: Container<Element = OutputScalar> + Sync,
    // Maybe Sized allows to pass Box<dyn Seeder>.
    NoiseSeeder: Seeder + ?Sized,
{
    let mut bsk = SeededLweBootstrapKeyOwned::new(
        OutputScalar::ZERO,
        output_glwe_secret_key.glwe_dimension().to_glwe_size(),
        output_glwe_secret_key.polynomial_size(),
        decomp_base_log,
        decomp_level_count,
        input_lwe_secret_key.lwe_dimension(),
        noise_seeder.seed().into(),
        ciphertext_modulus,
    );

    par_generate_seeded_lwe_bootstrap_key(
        input_lwe_secret_key,
        output_glwe_secret_key,
        &mut bsk,
        noise_distribution,
        noise_seeder,
    );

    bsk
}
