use std::io::{Cursor};

use rand::thread_rng;
use rand::{Rng, distributions::Alphanumeric};
use base64;
use byteorder::{NativeEndian, WriteBytesExt, ReadBytesExt};
use crate::errors::LabraError;

use std::iter::repeat;
use openssl::{symm};
use openssl::hash::{MessageDigest};
use openssl::pkey::PKey;
use openssl::rsa::{Padding, Rsa};
use openssl::sign::{Signer, Verifier};
use rustc_serialize::hex::{ToHex, FromHex};
use crate::LabradorResult;

#[allow(unused)]
pub enum HashType {
    Sha1,
    Sha256
}

#[derive(Debug, Eq, PartialEq)]
pub struct PrpCrypto {
    key: Vec<u8>,
}


#[allow(unused)]
/// 加密相关
impl PrpCrypto {
    pub fn new(key: Vec<u8>) -> PrpCrypto {
        PrpCrypto {
            key,
        }
    }

    /// 随机字符串
    fn get_random_string() -> String {
        if cfg!(test) {
            "1234567890123456".to_owned()
        } else {
            thread_rng().sample_iter(&Alphanumeric).take(16).collect::<String>()
        }
    }

    /// # 加密消息(aes_128_cbc)
    pub fn aes_128_cbc_encrypt_msg(&self, plaintext: &str, _id: &str) -> LabradorResult<String> {
        let mut wtr = PrpCrypto::get_random_string().into_bytes();
        wtr.write_u32::<NativeEndian>((plaintext.len() as u32).to_be()).unwrap_or_default();
        wtr.extend(plaintext.bytes());
        wtr.extend(_id.bytes());
        let encrypted = symm::encrypt(symm::Cipher::aes_128_cbc(), &self.key, Some(&self.key[..16]), &wtr)?;
        let b64encoded = base64::encode(&encrypted);
        Ok(b64encoded)
    }

    /// # 解密消息(aes_128_cbc)
    pub fn aes_128_cbc_decrypt_msg(&self, ciphertext: &str, _id: &str) -> LabradorResult<String> {
        let b64decoded = base64::decode(ciphertext)?;
        let text = symm::decrypt(symm::Cipher::aes_128_cbc(), &self.key, Some(&self.key[..16]), &b64decoded)?;
        let mut rdr = Cursor::new(text[16..20].to_vec());
        let content_length = u32::from_be(rdr.read_u32::<NativeEndian>().unwrap_or_default()) as usize;
        let content = &text[20 .. content_length + 20];
        let from_id = &text[content_length + 20 ..];
        if from_id != _id.as_bytes() {
            return Err(LabraError::InvalidAppId);
        }
        let content_string = String::from_utf8(content.to_vec()).unwrap_or_default();
        Ok(content_string)
    }


    /// # 解密数据(aes_128_cbc)
    pub fn aes_128_cbc_decrypt_data(&self, ciphertext: &str, iv: &str) -> LabradorResult<String> {
        let data = ciphertext.from_hex()?;
        let text = symm::decrypt(symm::Cipher::aes_128_cbc(), &self.key, Some(iv.as_bytes()), &data)?;
        let content_string = String::from_utf8(text).unwrap_or_default();
        Ok(content_string)
    }


    /// # 加密数据(aes_128_cbc)
    pub fn aes_128_cbc_encrypt_data(&self, plaintext: &str, iv: &str) -> LabradorResult<String> {
        let text = symm::encrypt(symm::Cipher::aes_128_cbc(), &self.key, Some(iv.as_bytes()), plaintext.as_bytes())?;
        Ok(text.to_hex())
    }

    /// RSA签名
    ///
    /// - content: 签名内容
    /// - private_key: 私钥，PKCS#1
    /// - hash_type: hash类型
    ///
    /// # Examples
    ///
    /// ```
    /// let content = "123";
    /// let private_key = "your private key";
    /// let sign = rsa_sign(content, private_key);
    ///
    /// println!("sign:{}", sign);
    /// ```
    /// return: 返回base64字符串
    pub fn rsa_sha256_sign(content: &str, private_key: &str) -> LabradorResult<String> {
        let private_key = openssl::rsa::Rsa::private_key_from_pem(private_key.as_bytes())?;
        let pkey = PKey::from_rsa(private_key)?;
        let mut signer = Signer::new(MessageDigest::sha256(), &pkey).unwrap();
        signer.set_rsa_padding(Padding::PKCS1)?;
        signer.update(content.as_bytes())?;
        let result = signer.sign_to_vec()?;
        // 签名结果转化为base64
        Ok(base64::encode(&result))
    }

    pub fn rsa_sha256_sign_pkcs1(content: &str, private_key: Vec<u8>) -> LabradorResult<String> {
        let private_key = openssl::rsa::Rsa::private_key_from_der(&private_key)?;
        let pkey = PKey::from_rsa(private_key)?;
        let mut signer = Signer::new(MessageDigest::sha256(), &pkey)?;
        signer.set_rsa_padding(Padding::PKCS1)?;
        signer.update(content.as_bytes())?;
        let result = signer.sign_to_vec()?;
        // 签名结果转化为base64
        Ok(base64::encode(&result))
    }

    pub fn rsa_sha256_sign_pkcs8(content: &str, private_key: Vec<u8>) -> LabradorResult<String> {
        let pkey = PKey::private_key_from_pkcs8(&private_key)?;
        let mut signer = Signer::new(MessageDigest::sha256(), &pkey)?;
        signer.update(content.as_bytes())?;
        let result = signer.sign_to_vec()?;
        // 签名结果转化为base64
        Ok(base64::encode(&result))
    }

    /// RSA签名验证
    /// 使用微信支付平台公钥对验签名串和签名进行SHA256 with RSA签名验证。
    /// - content: 签名内容
    /// - public_key: 公钥，PKCS#1
    /// - sign: 签名
    ///
    /// # Examples
    ///
    /// ```
    /// let content = "123";
    /// let public_key = "your public key";
    /// let sign = rsa_sign(public_key, content, sign);
    ///
    /// println!("sign:{}", sign);
    /// ```
    pub fn rsa_sha256_verify(public_key: &str, content: &str, sign: &str) -> LabradorResult<bool> {
        let sig = base64::decode(sign)?;
        let sig = sig.to_hex();
        let sig = sig.from_hex()?;
        // 获取公钥对象
        let pk = Rsa::public_key_from_pem(public_key.as_bytes())?;
        let pkey = PKey::from_rsa(pk)?;
        // 对摘要进行签名
        let mut verifier = Verifier::new(MessageDigest::sha256(), &pkey)?;
        verifier.update(content.as_bytes())?;
        let ver = verifier.verify(&sig)?;
        Ok(ver)
    }

    pub fn hmac_sha256_sign(key: &str, message: &str) -> LabradorResult<String> {
        let pkey = PKey::hmac(key.as_bytes())?;
        let mut signer = Signer::new(MessageDigest::sha256(), &pkey).unwrap();
        signer.update(message.as_bytes())?;
        let result = signer.sign_to_vec()?;
        Ok(result.to_hex())
    }

    /// # 加密(aes_256_gcm)
    pub fn aes_256_gcm_encrypt(&self, associated_data: &[u8], nonce: &[u8], plain_text: &[u8]) -> LabradorResult<Vec<u8>> {
        let mut out_tag: Vec<u8> = repeat(0).take(16).collect();
        let encrypted = symm::encrypt_aead(symm::Cipher::aes_256_gcm(), &self.key, Some(&nonce), associated_data, plain_text, &mut out_tag)?;
        Ok(encrypted)
    }

    /// # 解密(aes_256_gcm)
    pub fn aes_256_gcm_decrypt(&self, associated_data: &[u8], nonce: &[u8], ciphertext: &[u8], tag: &[u8]) -> LabradorResult<Vec<u8>> {
        let decrypted = symm::decrypt_aead(symm::Cipher::aes_256_gcm(), &self.key, Some(&nonce), associated_data, ciphertext, tag)?;
        Ok(decrypted)
    }
}

#[allow(unused, non_snake_case)]
#[cfg(test)]
mod tests {
    use std::iter::repeat;
    use base64;
    use super::PrpCrypto;
    use rustc_serialize::hex::{FromHex, ToHex};


    #[test]
    fn test_prpcrypto_encrypt() {
        let encoding_aes_key = "kWxPEV2UEDyxWpmPdKC3F4dgPDmOvfKX1HGnEUDS1aR=";
        let key = base64::decode(encoding_aes_key).unwrap_or_default();
        let prp = PrpCrypto::new(key);
        // let encrypted = prp.encrypt("test", "rust").unwrap();
        // assert_eq!("9s4gMv99m88kKTh/H8IdkNiFGeG9pd7vNWl50fGRWXY=", &encrypted);
    }

    #[test]
    fn test_prpcrypto_decrypt() {
        let encoding_aes_key = "kWxPEV2UEDyxWpmPdKC3F4dgPDmOvfKX1HGnEUDS1aR=";
        let key = base64::decode(encoding_aes_key).unwrap();
        let prp = PrpCrypto::new(key);
        // let decrypted = prp.decrypt("9s4gMv99m88kKTh/H8IdkNiFGeG9pd7vNWl50fGRWXY=", "rust").unwrap();
        // assert_eq!("test", &decrypted);
    }

    fn hex_to_bytes(raw_hex: &str) -> Vec<u8> {
        raw_hex.from_hex().ok().unwrap()
    }

    #[test]
    fn test_prpcrypto_decrypt_v3() {
        // let key = hex_to_bytes("feffe9928665731c6d6a8f9467308308");
        // let iv= hex_to_bytes("cafebabefacedbaddecaf888");
        // let plain_text= hex_to_bytes("d9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a721c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b39");
        // let cipher_text= hex_to_bytes("42831ec2217774244b7221b784d0d49ce3aa212f2c02a4e035c17e2329aca12e21d514b25466931c7d8f6a5aac84aa051ba30b396a0aac973d58e091");
        // let aad= hex_to_bytes("feedfacedeadbeeffeedfacedeadbeefabaddad2");
        // let tag= hex_to_bytes("5bc94fbc3221a5db94fae95ae7121a47");
        // let key_size = match key.len() {
        //     16 => aes::KeySize::KeySize128,
        //     24 => aes::KeySize::KeySize192,
        //     32 => aes::KeySize::KeySize256,
        //     _ => unreachable!()
        // };
        // let mut decipher = AesGcm::new(key_size, &key[..], &iv[..], &aad[..]);
        // let mut out: Vec<u8> = repeat(0).take(plain_text.len()).collect();
        //
        // let result = decipher.decrypt(&cipher_text[..], &mut out[..], &tag[..]);
        // // let res = PrpCrypto::aes_gcm_decrypt(&aad, &iv, &cipher_text, &key);
        //
        // println!("test:{}",out.to_hex());

        let key = b"364ae33e57cf4989b8aefaa66ddc7ca7";
        let iv= b"bb9ee5e44da1";
        // let plain_text= hex_to_bytes("d9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a721c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b39");
        let cipher_text_base64=base64::decode("WZnvm4CnxNuPUYLIAh3Kv2WJFivwhLA2/xGxhwNHh5j2XmhUn2ibLm1I/pU3XKw6YWYLY8RfHsRHVcY4ln0NUUsiqsmgUxELKjqPKY0dWZSwXtbVAMlK+rGQbrgoopn/gNurM6Sx0jOjzorg091J0GGkxn2hHSaJ6EUtbHAGB3Nx/PTLr2o1rzNvF/QWLGE+5bcGe5Yg85qshvoGATJSwNAlVmdCOV4fg583irGzg6u7MYAytZpBoyzA4yf+9AKrO3K5lQwF5G6ULPWXtTNuW4rrC8wPI5xdnLqKopo9gNDUqg+19DYDSYsUvztRU7wORNh0SVkZLTwhOmKzFM8oqDHDuvcRCrUjw52NT85BQIFtsJMHciiFL+pefsz1llxlDnjroRyqNAyXw0RvKJfff40M8Fw7mAWK5eINQLPZAi4f9Ws7vC3WZ9/WGjrPOQInn8oLxzb8c+Wn0HSAxfEBRBmGx8FQ0+MdAP5bHTn3KCVxBM8gdx5vfeNqzcnRPG6qTMwuf/NE4BdnqNsDk5o3ZyhMGxnDfoJ+9PophG5KtdaPYHDVj/18PzT0w4GttSdw/1pisSPeOKcQqpI3/sC3ndDO7uqieUUAhMCtLxFCn1spndDLr+ciUs3CWJYlBgATE8vOFzPjVN8ECV+UeGULjkjWGBm0yPG3znbBpkX5Zvei4eZml16/JZHTWVgAKHpaaoBNH6qLKqS4UdpAXZJEQLAXflRw+4RjyD8ZsERcOTutnycozb/sPxB8N3qWhTGb8EJ8DTYSCILYemSIDmefmPU+ChzdM1FDbePMpHv8wCC/+zfRSwl0VtWXCauazZ3+1J9dW8ThvTOwlXPuRvOXFwCX/bq8BI3DX619TnahNBKU3+EfcvGGDO6bI5LvPSPLAaf1MgPc31Ab4jP+s73y4vc5IYNuwMC+aKuPmaxrqPA6Lr7PAUEicem4mYiTOAeG4hQh2C9XSOKrocsNDaOgLRiUU53bNY9sBTEkxoOc5prYVV7azwPfR506fSec0fv5c7v58srSK9zpTKNNVKbLL76WCpQ453dwmyaYeJNVqYoslzEL+kcb6UZVwr/Kj9TJka5bYHQOBmTRJT7FUeawvu4kHWzWnlRUShNFkuoymJEA8SXYyPliJgBWl36HAWse3PNr63K+RoYe8VdtviQQ02Js2Bg2RcTAlaxSoKuQdFfraGh35gVeJYEbrIp3N5goxLc6oc+bE/uoQI+pgv6oNsNznotp7bPCY1hIOEdtgvxMAUnpiU5ZsiPGt/N5KVAvSZJMzbuql3p2LBZjY3aGsNsT+xfgMj9K1fsORHP8/zt+RoF3AasSnn66zWRlxGlptkH+HtNxfEefaHtZ3NwYNPwaKwn9hIF5EotIhgLRsbEL9PWJLBVDuaWcmoaYDTNzAUlpGAKvyh2e4U7j3VuxPDiwNmPC+ZG/2CSMuD3+GPJodA3wbkhiNP4TAitKgYC03i94HDj8i2Th5HvNuA+dap7LaZerV7A34DwCK4rwk2C6z8+TAhdqagv2q1rnvzVT/dUXkIz3YMNkowboTpc/VgENPgUGBM4TtUpdk+hSxx/L5q/C+uWt8U1rIxbu5JrN3dHlvF/WfaCHQZP8e2QC8bz/TSX/tzFIQ6o/QtFWlF8OGbbndoNgTe5xyS5AwlprmR9FWFzjim8JAKNKMTKTrW3U6TKSUxSD9m7sl08rD3pCk+1kkKiVEgcuVHPd985n1xr4Ex9Hr8pJBTDcbkzis+dvh+CajqgsrYas+Eq8NTM8pz004PcPfZZzuaLgjl0Z+l7ZschSCkzq54BRxfIcvwywqJUhtRmB6xccpCtln6AsC/FS+kcJdAYEnnuU5uoPmNCcf3n+jDL9UGbcNg5Nj/w92tyF5A==").unwrap();
        let base64_cipher = cipher_text_base64.to_hex();
        println!("cipher_text:{}", &base64_cipher);
        let cipher_text = hex_to_bytes(&base64_cipher);
        let aad= b"certificate";

        let cipherdata_length = cipher_text.len() - 16;
        let cipherdata_bytes = &cipher_text[0..cipherdata_length];
        let tag = &cipher_text[cipherdata_length..cipher_text.len()];
        // let res = PrpCrypto::aes_gcm_encrypt(&aad, &iv, &plain_text, &key).unwrap();
        // println!("aes_gcm_encrypt result:{}", res.to_hex());
        //
        // let res = PrpCrypto::aes_gcm_decrypt(aad, iv, cipherdata_bytes, key, tag).unwrap();
        // println!("aes_gcm_decrypt result:{}", String::from_utf8_lossy(&res));

        // let key_size = match key.len() {
        //     16 => aes::KeySize::KeySize128,
        //     24 => aes::KeySize::KeySize192,
        //     32 => aes::KeySize::KeySize256,
        //     _ => unreachable!()
        // };
        // let mut decipher = AesGcm::new(key_size, &key[..], &iv[..], &aad[..]);
        // let mut out: Vec<u8> = repeat(0).take(ctxet.len()).collect();
        //
        // let result = decipher.decrypt(&ctxet[..], &mut out[..], &tag[..]);
        // // let res = PrpCrypto::aes_gcm_decrypt(&aad, &iv, &cipher_text, &key);
        // println!("res:{},test:{}",result, out.to_hex());
    }

    #[test]
    fn test_check_decrypted_data_should_ok() {
        let appId = "wx4f4bc4dec97d474b";
        let encoding_aes_key = "kWxPEV2UEDyxWpmPdKC3F4dgPDmOvfKX1HGnEUDS1aR=";
        let sessionKey = "d5k+F2N8DJ1K7+O2YNCH+g==";
        let encryptedData = "RfBSVSlEmUxa7rHkJqPZivUhsvBPX/HtkNFkyJYYMn77tid0laa+qSi/G5Bd027JbzQaKW2q3Qqjppm9NGwp7hdqaGfChAma6wqkWsoh7BmouVcX46u1rNNBKNZbJJuKjjzS+cVUEeiVjOZE6iCvEH/XzKqf1dSFO1FDKu+MAkS0ScOB3zFplR48Y/Q30VHm5/rlYsLkuxULHxb78tcMiCAAsp5uuac+wDC+Ehof5n8NT/g6PFO77Tpf1Qykx5wXSI2rZj1xHDCsfJ2/K0Vf/bj0prGEwXd7HcuKJiZqrqEUBQcBk6ji000oQ1lQKNAp0YofFv8E2lINQgkJEdvo4mDw1v3/CaJNmriJ0jAE2g4bmfCyp6cY3HMX3o0zLLbCKFSwd8IhTSxBDNuXgxOX+sz0px9mS9CcFpUOIhLJQdOFqTr5fjqzGMYcp4mPs6HS0L4Zw8lMqYranA2vSlWCCyCt7AmPzTMlJZn9yi9PBmg=";
        let iv = "SRETvbQYX07NpMDK9kZOQw==";
        let key = base64::decode(sessionKey).unwrap();
        let prp = PrpCrypto::new(key);
        // match prp.decrypt_data(encryptedData, iv) {
        //     Ok(data) => {
        //         println!("data:{}",data);
        //     }
        //     Err(err) => {
        //         println!("err:{:?}",err);
        //     }
        // }
    
    }

    #[test]
    fn test_aes_128_ecb() {
        let appId = "1ebc3d10ce15cf8cc601f60d3e84385c4d7acc9cc70fcd56dbbd969300c8f6082625cdd2cf66738f4635406a4c796bf7e1769d7ccfb468537ba211bdbf8fb13e09c343f52b1f5a47cab44126b61e338acc93b4cc12939a131f7b15a1af54be699dbb7ce3770aa8261af253d2aeac41c1c2db333d0052b48de4e58541bab56d98";
        let key = base64::decode("4ChT08phkz59hquD795X7w==").unwrap();
        let prp = PrpCrypto::new(key);
        println!("result:{}", prp.aes_128_cbc_decrypt_data(appId, "dsd2bb9ee5e44da1").unwrap());
        // match prp.decrypt_data(encryptedData, iv) {
        //     Ok(data) => {
        //         println!("data:{}",data);
        //     }
        //     Err(err) => {
        //         println!("err:{:?}",err);
        //     }
        // }

    }
}
