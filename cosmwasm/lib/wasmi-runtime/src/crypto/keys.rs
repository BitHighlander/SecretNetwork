use enclave_ffi_types::CryptoError;
use secp256k1::ecdh::SharedSecret;
use secp256k1::key::{PublicKey, SecretKey};
use secp256k1::{All, Secp256k1};
use sgx_trts::trts::rsgx_read_rand;

pub const SEED_KEY_SIZE: usize = 32;

pub const PUBLIC_KEY_SIZE: usize = 64;
/// The size of the symmetric 256 bit key we use for encryption (in bytes).
pub const SYMMETRIC_KEY_SIZE: usize = 256 / 8;
/// The size of the master seed
pub const SEED_SIZE: usize = 32;
/// The size of secret keys
pub const SECRET_KEY_SIZE: usize = secp256k1::constants::SECRET_KEY_SIZE;
/// The size of uncomressed public keys
pub const UNCOMPRESSED_PUBLIC_KEY_SIZE: usize = secp256k1::constants::UNCOMPRESSED_PUBLIC_KEY_SIZE;
/// symmetric key we use for encryption.
pub type SymmetricKey = [u8; SYMMETRIC_KEY_SIZE];
/// StateKey is the key used for state encryption.
pub type StateKey = SymmetricKey;
/// DHKey is the key that results from the ECDH [`enigma_crypto::KeyPair::derive_key`](../replace_me)
pub type DhKey = SymmetricKey;
/// PubKey is a public key that is used for ECDSA signing.
pub type PubKey = [u8; UNCOMPRESSED_PUBLIC_KEY_SIZE];

#[derive(Debug)]
pub struct AESKey(SymmetricKey);

impl AESKey {
    pub fn get(&self) -> &[u8; SYMMETRIC_KEY_SIZE] {
        return &self.0;
    }

    pub fn new_from_slice(privkey: &[u8; SYMMETRIC_KEY_SIZE]) -> Self {
        Self { 0: privkey.clone() }
    }
}

pub struct Seed([u8; SEED_SIZE]);

impl Seed {
    pub fn get(&self) -> &[u8; SEED_SIZE] {
        return &self.0;
    }

    pub fn new_from_slice(s: &[u8; SEED_SIZE]) -> Self {
        Self { 0: s.clone() }
    }

    pub fn new() -> Result<Self, CryptoError> {
        let mut sk_slice = [0; SEED_SIZE];
        rand_slice(&mut sk_slice)?;
        Ok(Self::new_from_slice(&sk_slice))
    }
}
pub struct KeyPair {
    context: Secp256k1<All>,
    pubkey: PublicKey,
    privkey: SecretKey,
}

impl KeyPair {
    /// This will generate a fresh pair of Public and Private keys.
    /// it will use the available randomness from [crate::rand]
    pub fn new() -> Result<Self, CryptoError> {
        // This loop is important to make sure that the resulting public key isn't a point in infinity(at the curve).
        // So if the Resulting public key is bad we need to generate a new random private key and try again until it succeeds.
        loop {
            let context = Secp256k1::new();
            let mut sk_slice = [0; SECRET_KEY_SIZE];
            rand_slice(&mut sk_slice)?;
            if let Ok(privkey) = SecretKey::from_slice(&sk_slice) {
                let pubkey = PublicKey::from_secret_key(&context, &privkey);
                return Ok(KeyPair {
                    context,
                    privkey,
                    pubkey,
                });
            }
        }
    }

    /// This function will create a Pair of keys from an array of 32 bytes.
    /// Please don't use it to generate a new key, if you want a new key use `KeyPair::new()`
    /// Because `KeyPair::new()` will make sure it uses a good random source and will loop private keys until it's a good key.
    /// (and it's best to isolate the generation of keys to one place)
    pub fn new_from_slice(privkey: &[u8; SECRET_KEY_SIZE]) -> Result<Self, CryptoError> {
        let context = Secp256k1::new();

        let privkey = SecretKey::from_slice(privkey).map_err(|e| CryptoError::KeyError {})?;
        let pubkey = PublicKey::from_secret_key(&context, &privkey);

        Ok(KeyPair {
            context,
            privkey,
            pubkey,
        })
    }

    /// This function does an ECDH(point multiplication) between one's private key and the other one's public key
    pub fn derive_key(&self, pubarr: &PubKey) -> Result<DhKey, CryptoError> {

        // Pubkey is already 65 bytes, not sure what this is for?

        // let mut pubarr = [0; UNCOMPRESSED_PUBLIC_KEY_SIZE];
        // pubarr[0] = 4;
        // pubarr[1..].copy_from_slice(&_pubarr[..]);

        let pubkey = PublicKey::from_slice(pubarr).map_err(|e| CryptoError::KeyError {})?;

        let shared = SharedSecret::new(&pubkey, &self.privkey);

        let mut result = [0u8; SYMMETRIC_KEY_SIZE];
        result.copy_from_slice(shared.as_ref());
        Ok(result)
    }

    /// This will return the raw 32 bytes private key. use carefully.
    pub fn get_privkey(&self) -> &[u8] {
        &self.privkey[..]
    }

    // This will return the raw 64 bytes public key.
    pub fn get_pubkey(&self) -> PubKey {
        self.pubkey.serialize_uncompressed()
    }
}

fn rand_slice(rand: &mut [u8]) -> Result<(), CryptoError> {
    rsgx_read_rand(rand).map_err(|e| CryptoError::RandomError {})
}
