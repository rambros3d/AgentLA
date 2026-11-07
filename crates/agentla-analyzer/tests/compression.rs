use agentla_analyzer::compression::{
    bit_delta_compress_block, bit_delta_decompress_block, bit_delta_transform_u32,
    bit_delta_untransform_u32, rle_decode, rle_encode,
};

#[test]
fn test_rle_round_trip() {
    let src = [1u8, 1, 1, 2, 3, 3, 3, 3, 4, 5, 5, 5, 5, 5, 5];
    let mut encoded = [0u8; 64];
    let enc_len = rle_encode(&src, &mut encoded, 64);
    assert!(enc_len > 0);

    let mut decoded = [0u8; 64];
    let dec_len = rle_decode(&encoded[..enc_len], &mut decoded, 64);
    assert_eq!(dec_len, src.len());
    assert_eq!(&decoded[..dec_len], &src);
}

#[test]
fn test_bit_delta_transform_round_trip() {
    let original = [0x12345678u32, 0x12345678, 0x9ABCDEF0, 0x00000000, 0xFFFFFFFF];
    let mut delta = [0u32; 5];
    let mut prev_state = 0u32;
    bit_delta_transform_u32(&original, &mut delta, &mut prev_state);

    let mut restored = [0u32; 5];
    let mut prev_state_dec = 0u32;
    bit_delta_untransform_u32(&delta, &mut restored, &mut prev_state_dec);

    assert_eq!(&restored, &original);
}

#[test]
fn test_bit_delta_block_compression_round_trip() {
    // 1. Constant runs
    let mut block1 = [0xAA55AA55u32; 128];
    block1[10] = 0xAA55AA54;
    block1[50] = 0x00000000;

    let mut compressed = [0u8; 1024];
    let comp_len = bit_delta_compress_block(&block1, &mut compressed);
    assert!(comp_len > 0);
    // Compression should achieve significant space savings on runs
    assert!(comp_len < block1.len() * 4);

    let mut decompressed = [0u32; 128];
    let decomp_len = bit_delta_decompress_block(&compressed[..comp_len], &mut decompressed);
    assert_eq!(decomp_len, block1.len());
    assert_eq!(&decompressed, &block1);
}

#[test]
fn test_bit_delta_block_clock_toggle() {
    let mut block = [0u32; 64];
    for i in 0..64 {
        block[i] = if i % 2 == 0 { 0x01 } else { 0x00 };
    }

    let mut compressed = [0u8; 512];
    let comp_len = bit_delta_compress_block(&block, &mut compressed);
    assert!(comp_len > 0);

    let mut decompressed = [0u32; 64];
    let decomp_len = bit_delta_decompress_block(&compressed[..comp_len], &mut decompressed);
    assert_eq!(decomp_len, block.len());
    assert_eq!(&decompressed, &block);
}
