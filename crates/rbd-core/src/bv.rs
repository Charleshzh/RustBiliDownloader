//! BV <-> AV 双向转换算法.
//!
//! **算法来源**: 公开标准算法, 与 BBDown/Yutto/Pybilibili/social-sister-yi/bilibili-API-collect 一致.
//! 参考: <https://socialsisteryi.github.io/bilibili-API-collect/docs/misc/bv_id.html>

/// XOR 掩码 (B 站固定常量, 不可修改)
const XOR_CODE: u64 = 23_442_827_791_579;
/// 位掩码
const MASK_CODE: u64 = (1u64 << 51) - 1;
/// AV 号最大值
const MAX_AID: u64 = MASK_CODE + 1;
/// AV 号最小值
const MIN_AID: u64 = 1;
/// Base58 字符表 (固定顺序)
const ALPHABET: &[u8; 58] = b"FcwAPNKTMug3GV5Lj7EJnHpWsx4tb8haYeviqBz6rkCy12mUSDQX9RdoZf";
/// BV 负载长度 (不含 `BV1` 前缀)
const BV_PAYLOAD_LEN: usize = 9;

/// 校验 BV 号格式 (12 字符, 全在 ALPHABET 中).
pub fn validate_bv(bv: &str) -> anyhow::Result<()> {
    if bv.len() != 12 {
        anyhow::bail!("BV 号必须 12 字符, 实际 {} 字符: {bv}", bv.len());
    }
    if !bv.starts_with("BV1") {
        anyhow::bail!("BV 号必须以 'BV1' 开头: {bv}");
    }
    if !bv.chars().all(|c| ALPHABET.contains(&(c as u8))) {
        anyhow::bail!("BV 号含非法字符: {bv}");
    }
    Ok(())
}

/// AV 号 → BV 号.
///
/// # 性能
/// O(9) — 仅 9 步查表, < 1μs.
pub fn av_to_bv(av: u64) -> anyhow::Result<String> {
    if av < MIN_AID {
        anyhow::bail!("AV 号必须 >= 1: {av}");
    }
    if av >= MAX_AID {
        anyhow::bail!("AV 号超出 51 位范围: {av}");
    }
    let mut tmp = (MAX_AID | av) ^ XOR_CODE;
    let mut payload = [ALPHABET[0]; BV_PAYLOAD_LEN];

    for idx in (0..BV_PAYLOAD_LEN).rev() {
        payload[idx] = ALPHABET[(tmp % 58) as usize];
        tmp /= 58;
    }

    payload.swap(0, 6);
    payload.swap(1, 4);

    Ok(format!("BV1{}", String::from_utf8_lossy(&payload)))
}

/// BV 号 → AV 号.
pub fn bv_to_av(bv: &str) -> anyhow::Result<u64> {
    validate_bv(bv)?;
    let mut payload = bv.as_bytes()[3..].to_vec();
    payload.swap(0, 6);
    payload.swap(1, 4);

    let mut avid = 0u64;
    for byte in payload {
        let pos = ALPHABET
            .iter()
            .position(|&b| b == byte)
            .ok_or_else(|| anyhow::anyhow!("BV 号含非法字符: {bv}"))? as u64;
        avid = avid * 58 + pos;
    }

    Ok((avid & MASK_CODE) ^ XOR_CODE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_cases() {
        // 公开测试向量 (来自 social-sister-yi/bilibili-API-collect)
        assert_eq!(av_to_bv(170001).unwrap(), "BV17x411w7KC");
        assert_eq!(bv_to_av("BV17x411w7KC").unwrap(), 170001);
        assert_eq!(av_to_bv(4567890).unwrap(), "BV1gs411B7Mu");
        assert_eq!(bv_to_av("BV1gs411B7Mu").unwrap(), 4567890);
    }

    #[test]
    fn test_round_trip() {
        for av in [1, 100, 10000, 999999, 1000000, 100000000] {
            let bv = av_to_bv(av).unwrap();
            let back = bv_to_av(&bv).unwrap();
            assert_eq!(av, back, "round-trip failed for {av}");
        }
    }

    #[test]
    fn test_validate() {
        assert!(validate_bv("BV17x411w7KC").is_ok());
        assert!(validate_bv("BV1xx").is_err());
        assert!(validate_bv("XX17x411w7KC").is_err());
    }
}
