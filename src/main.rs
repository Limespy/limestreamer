#![feature(convert_float_to_int)]

use half::f16;
use std::time::Instant;

const TOLERANCE: f32 = 5.;
const IMAGE_WIDTH: usize = 500;
const IMAGE_HEIGHT: usize = 500;

const MAX_STEP_E: u8 = 0b111;
const EMBEDD_STENCIL: u8 = !MAX_STEP_E;
const MIN_STEP: usize = 2;
const MIN_STEP_E: u8 = 0;

const COMPRESSED_BUFFER_LEN: usize = IMAGE_WIDTH / MIN_STEP * 2 + 2;

const MAX_DELTA_VALUE_F32: f32 = u8::MAX as f32;
// const MAX_DELTA_VALUE_I16: i16 = MAX_DELTA_VALUE_F32 as i16;

const MAX_RAW_IMAGE_VALUE_U16: u16 = (1 << 12) - 1;
// const MAX_RAW_IMAGE_VALUE_I16: i16 = MAX_RAW_IMAGE_VALUE_U16 as i16;
const MAX_RAW_IMAGE_VALUE_F32: f32 = MAX_RAW_IMAGE_VALUE_U16 as f32;

const TEST_LOW_RAW_IMAGE_VALUE_I16: i16 = 485; // not specific
const TEST_LOW_RAW_IMAGE_VALUE_F32: f32 = TEST_LOW_RAW_IMAGE_VALUE_I16 as f32;
const TEST_HIGH_RAW_IMAGE_VALUE_I16: i16 = 3500;
const TEST_HIGH_RAW_IMAGE_VALUE_F32: f32 = TEST_HIGH_RAW_IMAGE_VALUE_I16 as f32; 
// const TEST_RANGE_RAW_IMAGE_VALUE: i16 = TEST_HIGH_RAW_IMAGE_VALUE_I16 - TEST_LOW_RAW_IMAGE_VALUE_I16;

const WAVELENGTH: f32 = 100.;
const PI: f32 = 3.14159;
// const WAVE_MULTIPLIER: f32 = 2. * PI / WAVELENGTH;


struct Debug {
    row_accesses: u32,
}

const SQRT_TABLE: [f32; MAX_STEP_E as usize + 1] = [1., 1., 2., 4., 5., 8., 11., 16.];
// // =====================================================================
// fn sqrt_std(n: f32) -> f32{
//     n.sqrt().floor()
// }
// // =====================================================================
// fn sqrt_poly(n: f32) -> f32{
//     const P2: f32 = -0.0012847881087919041;
//     const P1: f32 = 0.18251897533206835;
//     const P0: f32 = 0.785321790006325;
//     (n * (n * P2 + P1) + P0).floor()
// }
// =====================================================================
// #![feature(isqrt)]
// fn sqrt_i(n: f32) -> f32{
//     (n as u16).isqrt() as f32
// }
// =====================================================================
// fn embed(wrapper: f16, payload: u8) -> f16 {
//     let mut a: [u8; 2] =  wrapper.to_le_bytes();
//     a[0] &= EMBEDD_STENCIL;
//     a[0] |= payload;
//     f16::from_le_bytes(a)
// }
// =====================================================================
fn encode(y: f32, step_e: u8, compressed: &mut [u8], index: &mut usize) {
    let a: [u8; 2] =  f16::from_f32(y).to_le_bytes();
    compressed[*index] = (a[0] & EMBEDD_STENCIL) | step_e;
    compressed[*index+1] = a[1];
    *index += 2;
}
// =====================================================================
// fn decode(lsB: u8, step_e: u8, compressed: &mut [u8], index: &mut usize) {
//     let a: [u8; 2] =  f16::from_f32(y).to_le_bytes();
//     compressed[*index] = (a[0] & EMBEDD_STENCIL) | step_e;
//     compressed[*index+1] = a[1];
//     *index += 2;
// }
// =====================================================================
fn check(
    row: &[f32],
    index_start: usize,
    index_end: usize,
    y_start: f32,
    dy: f32,
    substep: usize,
    debug: &mut Debug
         ) -> bool {
    //! return true if check fails, false if not
    let mut fit: f32 = y_start;

    let mut index_check: usize = index_start + substep;

    while index_check <= index_end {

        fit += dy;

        debug.row_accesses += 1;
        if (fit - row[index_check]).abs() > TOLERANCE {
            return true; // marker value
        }
        index_check += substep;
    }
    false
}
// =====================================================================
fn compress(row: &[f32], compressed: &mut[u8]) -> (usize, Debug) {

    let mut debug = Debug{row_accesses: 0};

    const MIN_LENGTH: f32       = MIN_STEP as f32;
    const INITIAL_STEP: usize   = MIN_STEP * 2;
    const INITIAL_STEP_E: u8    = MIN_STEP_E + 1;
    const INITIAL_LENGTH: f32   = MIN_LENGTH * 2.;
    const INITIAL_SUBSTEP: f32  = INITIAL_LENGTH / 2.;

    let mut index_start: usize  = 0;
    let mut index_end: usize    = index_start + INITIAL_STEP;
    let row_end: usize          = row.len() - 1;

    
    let mut substep: f32;
    let mut step_valid: usize;
    let mut length_try: f32;
    let mut step_e_valid: u8;

    let mut y_start: f32        = row[index_start];
    let mut y_end: f32;
    let mut y_end_valid: f32;
    let mut y_mid: f32;

    let mut index_next_compressed: usize = 0;

    debug.row_accesses += 1;
    encode(y_start,
        0,
        compressed,
        &mut index_next_compressed);


    while index_end <= row_end {
        length_try      = INITIAL_LENGTH;
        step_valid      = MIN_STEP;
        substep         = INITIAL_SUBSTEP;

        y_end = row[index_end];
        debug.row_accesses += 1;
        y_end_valid = y_end;

        y_mid = row[index_start + MIN_STEP];
        debug.row_accesses += 1;

        if (y_start + (y_end - y_start) / length_try * substep - y_mid
            ).abs() < TOLERANCE {
            step_e_valid = INITIAL_STEP_E;

            while step_e_valid < MAX_STEP_E {

                index_end += step_valid; // doubles the step so far

                if index_end > row_end {
                    index_end = row_end;
                    // this should be the current
                    length_try = (index_end - index_start) as f32;
                } else {
                    length_try *= 2.; // this should be the current
                }
                y_end_valid = y_end;
                y_end = row[index_end];
                debug.row_accesses += 1;

                substep = SQRT_TABLE[(step_e_valid + 1) as usize]; //length.sqrt().floor();

                if check(row,
                    index_start,
                    index_end,
                    y_start,
                    (y_end - y_start) / length_try * substep,
                    substep as usize,
                    &mut debug) {
                    // Fit was not good enough
                    break
                }
                // the check passed, let's update valid values
                y_end_valid = y_end;
                step_valid <<= 1; // this should become the latest valid
                step_e_valid += 1; // this should become the latest valid
            }
        } else {
            // println!("fail first");
            step_e_valid = MIN_STEP_E;
            y_end_valid = y_mid;
        }
        // saving compressed value
        encode(y_end_valid,
            step_e_valid,
            compressed,
            &mut index_next_compressed);
        // println!("Base {} {} {}", index_next_compressed, index_start, step_e_valid);
        // using the fitted value
        y_start     = y_end_valid;
        index_start += step_valid;
        index_end   = index_start + INITIAL_STEP;

    }
    if index_start < row_end {

        debug.row_accesses += 1;

        encode(row[row_end],
            MAX_STEP_E,
            compressed,
            &mut index_next_compressed);
        // println!("Extra {}", index_next_compressed);
    }
    (index_next_compressed, debug)
}
// // =====================================================================
// fn in_place_delta(reference: &[i16], row: &mut[i16]) {
//     for i in 0..row.len() {
//         row[i] -= reference[i];
//     }
// }
// =====================================================================
fn preprocess(row_in: &[i16],
    row_key: &[i16],
    low: i16,
    high: i16,
    scaler: f32,
    row_out: &mut[f32]) {

    for i in 0..row_in.len() {
        row_out[i] = ((row_in[i].clamp(low, high)
                      - row_key[i]) as f32) * scaler;
    }
}
// =====================================================================
fn make_test_frame(
    image_width: usize, image_height: usize, low: f32, high: f32, wavelength: f32) -> Vec<[i16; IMAGE_HEIGHT]> {
    let mut frame = vec![[0_i16; IMAGE_WIDTH]; IMAGE_HEIGHT];

    let scaler: f32 =  0.25 * (high - low);
    let wave_multiplier: f32 = 2. * PI / wavelength;
    let mut sin_h: f32;

    for h in 0..image_height {
        sin_h = ((h as f32) * wave_multiplier).sin();
        for w in 0..image_width {
            frame[h][w] = ((sin_h + ((w as f32) * wave_multiplier).sin()  + 2.
                            ) * scaler + low) as i16;
        }
    }
    frame
}
// // =====================================================================
// fn time_sqrt() {
//     use rand::distributions::{Distribution, Uniform};

//     let mut rng = rand::thread_rng();
//     let exp = Uniform::from(2..7);


//     let mut randarray: [f32; 1000] = [0.; 1000];
//     let repeats: u32 = u32::try_from(randarray.len()).unwrap();

//     const BASE: f32 = 2.;

//     for i in 0..randarray.len() {
//         randarray[i] = BASE.powf(exp.sample(&mut rng) as f32) - 1.;
//     }
//     // -----------------------------------------------------------------
    
//     let mut s: f32 = 0.;

//     let time = Instant::now();
//     for n in randarray {
//         s += sqrt_std(n);
//     }
//     let elapsed = time.elapsed();

//     println!("{}", s);
//     println!("STD: {:.2?}", elapsed/repeats);
//     // -----------------------------------------------------------------
    
//     let mut s: f32 = 0.;

//     let time = Instant::now();
//     for n in randarray {
//         s += sqrt_poly(n);
//     }
//     let elapsed = time.elapsed();

//     println!("{}", s);
//     println!("Poly: {:.2?}", elapsed/repeats);
//     // -----------------------------------------------------------------
//     let mut s: f32 = 0.;

//     let time = Instant::now();
//     for n in randarray {
//         s += sqrt_i(n);
//     }
//     let elapsed = time.elapsed();

//     println!("{}", s);
//     println!("Integer: {:.2?}", elapsed/repeats);
// }
// // =====================================================================
// fn time_in_place_delta(&reference, &mut row) {

//     let time = Instant::now();

//     in_place_delta(&reference, &mut row);
//     in_place_delta(&reference, &mut row);
//     in_place_delta(&reference, &mut row);
//     in_place_delta(&reference, &mut row);
//     in_place_delta(&reference, &mut row);
//     in_place_delta(&reference, &mut row);
//     in_place_delta(&reference, &mut row);
//     in_place_delta(&reference, &mut row);
//     in_place_delta(&reference, &mut row);
//     in_place_delta(&reference, &mut row);

//     let time_compress = time.elapsed()/10;
//     println!("Delta in {:.2?}", time_compress);
// }
// =====================================================================
fn time_full_image() {

    let frame = make_test_frame(IMAGE_WIDTH,
        IMAGE_HEIGHT,
        TEST_LOW_RAW_IMAGE_VALUE_F32,
        TEST_HIGH_RAW_IMAGE_VALUE_F32,
        WAVELENGTH);
    let frame_key = make_test_frame(IMAGE_WIDTH,
                IMAGE_HEIGHT,
                TEST_LOW_RAW_IMAGE_VALUE_F32,
                TEST_HIGH_RAW_IMAGE_VALUE_F32 * 0.5, // less than the max height
                WAVELENGTH);

    let scaler: f32 = MAX_DELTA_VALUE_F32 / (MAX_RAW_IMAGE_VALUE_F32 - TEST_LOW_RAW_IMAGE_VALUE_F32);

    let mut points: usize = 0;
    let mut temporary_row: [f32; IMAGE_WIDTH] = [0.; IMAGE_WIDTH];

    let mut compressed: [u8; COMPRESSED_BUFFER_LEN] = [0; COMPRESSED_BUFFER_LEN];
    // timing
    let time = Instant::now();

    for h in 0..IMAGE_HEIGHT {
        preprocess(&frame[h],
            &frame_key[h],
            TEST_LOW_RAW_IMAGE_VALUE_I16,
            TEST_HIGH_RAW_IMAGE_VALUE_I16,
            scaler,
            &mut temporary_row);
        points += compress(&temporary_row, &mut compressed).0;
    }
    let time_compress = time.elapsed();

    let compression_ratio: f32 = points as f32 / ((IMAGE_HEIGHT * IMAGE_WIDTH) as f32);

    println!("Frame preprocessing and compression {:.2?} compresion ratio {:.3}",
    time_compress, compression_ratio);
}
// // =====================================================================
fn main() {
    // -----------------------------------------------------------------
    // time_sqrt()
    // -----------------------------------------------------------------
    // let input = f16::from_f32(255.);
    // println!("Embed from {:.2?} to {:.2?}", input, embed(input, 7));
    // -----------------------------------------------------------------
    // const COMPRESSED_BUFFER_LEN: usize = IMAGE_WIDTH / 2 + 1;
    // let mut compressed: [f16; COMPRESSED_BUFFER_LEN] = [f16::from_f32(0.); COMPRESSED_BUFFER_LEN];

    // let time = Instant::now();

    // let (length, debug) = compress(&row, &mut compressed);
    // let _ = compress(&row, &mut compressed);
    // let _ = compress(&row, &mut compressed);
    // let _ = compress(&row, &mut compressed);
    // let _ = compress(&row, &mut compressed);
    // let _ = compress(&row, &mut compressed);
    // let _ = compress(&row, &mut compressed);
    // let _ = compress(&row, &mut compressed);
    // let _ = compress(&row, &mut compressed);
    // let _ = compress(&row, &mut compressed);

    // let time_row = time.elapsed()/10;
    // let time_frame = time_row * (IMAGE_WIDTH as u32);

    // println!("Row compression:\n  from {}\n  to {}\n  in {:.2?}\n  frame {:.2?}\n  row accesses {}",
    // IMAGE_WIDTH, length, time_row, time_frame, debug.row_accesses);
    // -----------------------------------------------------------------
    // time_in_place_delta()
    // -----------------------------------------------------------------
    time_full_image();

}
