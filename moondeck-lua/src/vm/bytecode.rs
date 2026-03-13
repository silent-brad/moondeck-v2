/// Bytecode operations for the Lua VM.
///
/// Instructions are 32-bit packed:
/// - bits 0–5:   opcode (6 bits, up to 64 opcodes)
/// - bits 6–13:  A operand (8 bits, 0–255)
/// - bits 14–22: B operand (9 bits, 0–511)
/// - bits 23–31: C operand (9 bits, 0–511)
///
/// Alternative layouts sharing the same 32-bit word:
/// - A + Bx  (18 bits unsigned, bits 14–31)
/// - A + sBx (18 bits signed via bias encoding)
/// - sBx only (for unconditional `Jmp`)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Op {
    // ── Load / Move ─────────────────────────────────────────────────────
    Move = 0,     // A B     : R[A] = R[B]
    LoadNil,      // A       : R[A] = nil
    LoadTrue,     // A       : R[A] = true
    LoadFalse,    // A       : R[A] = false
    LoadConst,    // A Bx    : R[A] = constants[Bx]

    // ── Table operations ────────────────────────────────────────────────
    NewTable,     // A B C   : R[A] = new table(array_size=B, hash_size=C)
    GetTable,     // A B C   : R[A] = R[B][R[C]]
    SetTable,     // A B C   : R[A][R[B]] = R[C]
    GetField,     // A B C   : R[A] = R[B][constants[C]]   (string key)
    SetField,     // A B C   : R[A][constants[C]] = R[B]    (string key)
    GetIndex,     // A B C   : R[A] = R[B][C]               (small int key)
    SetIndex,     // A B C   : R[A][B] = R[C]               (small int key)

    // ── Globals ─────────────────────────────────────────────────────────
    GetGlobal,    // A Bx    : R[A] = globals[constants[Bx]]
    SetGlobal,    // A Bx    : globals[constants[Bx]] = R[A]

    // ── Upvalues ────────────────────────────────────────────────────────
    GetUpval,     // A B     : R[A] = upvalues[B]
    SetUpval,     // A B     : upvalues[B] = R[A]

    // ── Arithmetic ──────────────────────────────────────────────────────
    Add,          // A B C   : R[A] = R[B] + R[C]
    Sub,          // A B C   : R[A] = R[B] - R[C]
    Mul,          // A B C   : R[A] = R[B] * R[C]
    Div,          // A B C   : R[A] = R[B] / R[C]
    Mod,          // A B C   : R[A] = R[B] % R[C]
    Unm,          // A B     : R[A] = -R[B]

    // ── String ──────────────────────────────────────────────────────────
    Concat,       // A B C   : R[A] = R[B] .. R[C]
    Len,          // A B     : R[A] = #R[B]

    // ── Comparison (result stored in R[A]) ──────────────────────────────
    Eq,           // A B C   : R[A] = (R[B] == R[C])
    Ne,           // A B C   : R[A] = (R[B] ~= R[C])
    Lt,           // A B C   : R[A] = (R[B] <  R[C])
    Le,           // A B C   : R[A] = (R[B] <= R[C])
    Gt,           // A B C   : R[A] = (R[B] >  R[C])
    Ge,           // A B C   : R[A] = (R[B] >= R[C])

    // ── Logic ───────────────────────────────────────────────────────────
    Not,          // A B     : R[A] = not R[B]

    // ── Test and jump (short-circuit `and` / `or`) ──────────────────────
    TestSet,      // A B C   : if (bool(R[B]) == C) then R[A] = R[B] else pc++

    // ── Jumps ───────────────────────────────────────────────────────────
    Jmp,          // sBx     : pc += sBx
    JmpIf,        // A sBx   : if R[A] is truthy, pc += sBx
    JmpIfNot,     // A sBx   : if R[A] is falsy,  pc += sBx

    // ── Function calls ──────────────────────────────────────────────────
    Call,         // A B C   : call R[A] with B-1 args, expect C-1 results
                  //           B=0 → args up to top; C=0 → return all
    Return,       // A B     : return B-1 values from R[A..A+B-2]; B=0 → to top

    // ── Numeric for loop ────────────────────────────────────────────────
    ForPrep,      // A sBx   : R[A] -= R[A+2]; pc += sBx  (setup)
    ForLoop,      // A sBx   : R[A] += R[A+2]; if R[A] <= R[A+1] { pc += sBx; R[A+3] = R[A] }

    // ── Closure ─────────────────────────────────────────────────────────
    Closure,      // A Bx    : R[A] = closure(proto[Bx])

    // ── Upvalue management ──────────────────────────────────────────────
    Close,        // A       : close all upvalues >= R[A]

    // ── Miscellaneous ───────────────────────────────────────────────────
    Nop,          // no-op (alignment / patching)
}

// ─── Bit-field layout constants ─────────────────────────────────────────────

const OPCODE_BITS: u32 = 6;
const A_BITS: u32 = 8;
const B_BITS: u32 = 9;
const C_BITS: u32 = 9;
const BX_BITS: u32 = B_BITS + C_BITS; // 18

const OPCODE_MASK: u32 = (1 << OPCODE_BITS) - 1;   // 0x3F
const A_MASK: u32 = (1 << A_BITS) - 1;              // 0xFF
const B_MASK: u32 = (1 << B_BITS) - 1;              // 0x1FF
const C_MASK: u32 = (1 << C_BITS) - 1;              // 0x1FF
const BX_MASK: u32 = (1 << BX_BITS) - 1;            // 0x3FFFF

const A_SHIFT: u32 = OPCODE_BITS;                   // 6
const B_SHIFT: u32 = OPCODE_BITS + A_BITS;           // 14
const C_SHIFT: u32 = OPCODE_BITS + A_BITS + B_BITS;  // 23
const BX_SHIFT: u32 = B_SHIFT;                       // 14

/// Bias applied to convert a signed sBx value into an unsigned Bx field.
/// sBx range: [-(SBX_OFFSET), +(SBX_OFFSET)]  →  stored as (sbx + SBX_OFFSET).
const SBX_OFFSET: i32 = (1 << (BX_BITS - 1)) as i32 - 1; // 131_071

// ─── Encoding ───────────────────────────────────────────────────────────────

/// Encode an ABC-format instruction: `op A B C`.
#[inline]
pub fn encode_abc(op: Op, a: u8, b: u16, c: u16) -> u32 {
    debug_assert!((b as u32) <= B_MASK, "B operand out of range");
    debug_assert!((c as u32) <= C_MASK, "C operand out of range");

    (op as u32 & OPCODE_MASK)
        | ((a as u32 & A_MASK) << A_SHIFT)
        | ((b as u32 & B_MASK) << B_SHIFT)
        | ((c as u32 & C_MASK) << C_SHIFT)
}

/// Encode an ABx-format instruction: `op A Bx` (unsigned 18-bit immediate).
#[inline]
pub fn encode_abx(op: Op, a: u8, bx: u32) -> u32 {
    debug_assert!(bx <= BX_MASK, "Bx operand out of range");

    (op as u32 & OPCODE_MASK)
        | ((a as u32 & A_MASK) << A_SHIFT)
        | ((bx & BX_MASK) << BX_SHIFT)
}

/// Encode an AsBx-format instruction: `op A sBx` (signed 18-bit immediate).
#[inline]
pub fn encode_asbx(op: Op, a: u8, sbx: i32) -> u32 {
    let biased = (sbx + SBX_OFFSET) as u32;
    debug_assert!(biased <= BX_MASK, "sBx operand out of range");

    (op as u32 & OPCODE_MASK)
        | ((a as u32 & A_MASK) << A_SHIFT)
        | ((biased & BX_MASK) << BX_SHIFT)
}

/// Encode an sBx-only instruction (A is unused, set to 0).
/// Used for unconditional `Jmp`.
#[inline]
pub fn encode_sbx(op: Op, sbx: i32) -> u32 {
    encode_asbx(op, 0, sbx)
}

// ─── Decoding ───────────────────────────────────────────────────────────────

/// Total number of opcodes (used for validation during decode).
const OP_COUNT: u8 = Op::Nop as u8 + 1;

/// Decode the opcode from an instruction word.
///
/// # Panics
/// Panics on an unrecognised opcode value (indicates a corrupt instruction).
#[inline]
pub fn decode_op(instr: u32) -> Op {
    let raw = (instr & OPCODE_MASK) as u8;
    assert!(raw < OP_COUNT, "invalid opcode: {raw}");
    // SAFETY: `Op` is `repr(u8)` with contiguous discriminants 0..OP_COUNT,
    // and we just verified `raw` is in range.
    unsafe { core::mem::transmute::<u8, Op>(raw) }
}

/// Decode the A operand (8-bit register index).
#[inline]
pub fn decode_a(instr: u32) -> u8 {
    ((instr >> A_SHIFT) & A_MASK) as u8
}

/// Decode the B operand (9-bit field).
#[inline]
pub fn decode_b(instr: u32) -> u16 {
    ((instr >> B_SHIFT) & B_MASK) as u16
}

/// Decode the C operand (9-bit field).
#[inline]
pub fn decode_c(instr: u32) -> u16 {
    ((instr >> C_SHIFT) & C_MASK) as u16
}

/// Decode the unsigned Bx operand (18-bit).
#[inline]
pub fn decode_bx(instr: u32) -> u32 {
    (instr >> BX_SHIFT) & BX_MASK
}

/// Decode the signed sBx operand (18-bit biased).
#[inline]
pub fn decode_sbx(instr: u32) -> i32 {
    ((instr >> BX_SHIFT) & BX_MASK) as i32 - SBX_OFFSET
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_abc() {
        // NewTable R[3] = new table(array=10, hash=20)
        let instr = encode_abc(Op::NewTable, 3, 10, 20);
        assert_eq!(decode_op(instr), Op::NewTable);
        assert_eq!(decode_a(instr), 3);
        assert_eq!(decode_b(instr), 10);
        assert_eq!(decode_c(instr), 20);
    }

    #[test]
    fn roundtrip_abc_max_values() {
        let instr = encode_abc(Op::Add, 255, 511, 511);
        assert_eq!(decode_op(instr), Op::Add);
        assert_eq!(decode_a(instr), 255);
        assert_eq!(decode_b(instr), 511);
        assert_eq!(decode_c(instr), 511);
    }

    #[test]
    fn roundtrip_abc_zero() {
        let instr = encode_abc(Op::Move, 0, 0, 0);
        assert_eq!(decode_op(instr), Op::Move);
        assert_eq!(decode_a(instr), 0);
        assert_eq!(decode_b(instr), 0);
        assert_eq!(decode_c(instr), 0);
    }

    #[test]
    fn roundtrip_abx() {
        // LoadConst R[5] = constants[1000]
        let instr = encode_abx(Op::LoadConst, 5, 1000);
        assert_eq!(decode_op(instr), Op::LoadConst);
        assert_eq!(decode_a(instr), 5);
        assert_eq!(decode_bx(instr), 1000);
    }

    #[test]
    fn roundtrip_abx_max() {
        let bx_max = BX_MASK;
        let instr = encode_abx(Op::GetGlobal, 255, bx_max);
        assert_eq!(decode_op(instr), Op::GetGlobal);
        assert_eq!(decode_a(instr), 255);
        assert_eq!(decode_bx(instr), bx_max);
    }

    #[test]
    fn roundtrip_asbx_positive() {
        // ForLoop R[0], jump forward 42
        let instr = encode_asbx(Op::ForLoop, 0, 42);
        assert_eq!(decode_op(instr), Op::ForLoop);
        assert_eq!(decode_a(instr), 0);
        assert_eq!(decode_sbx(instr), 42);
    }

    #[test]
    fn roundtrip_asbx_negative() {
        // ForLoop R[1], jump back -100
        let instr = encode_asbx(Op::ForLoop, 1, -100);
        assert_eq!(decode_op(instr), Op::ForLoop);
        assert_eq!(decode_a(instr), 1);
        assert_eq!(decode_sbx(instr), -100);
    }

    #[test]
    fn roundtrip_asbx_zero() {
        let instr = encode_asbx(Op::JmpIf, 7, 0);
        assert_eq!(decode_op(instr), Op::JmpIf);
        assert_eq!(decode_a(instr), 7);
        assert_eq!(decode_sbx(instr), 0);
    }

    #[test]
    fn roundtrip_asbx_extremes() {
        let max_sbx = SBX_OFFSET;
        let min_sbx = -SBX_OFFSET;

        let instr_max = encode_asbx(Op::ForPrep, 2, max_sbx);
        assert_eq!(decode_sbx(instr_max), max_sbx);

        let instr_min = encode_asbx(Op::ForPrep, 2, min_sbx);
        assert_eq!(decode_sbx(instr_min), min_sbx);
    }

    #[test]
    fn roundtrip_sbx_only() {
        // Jmp +50 (A unused)
        let instr = encode_sbx(Op::Jmp, 50);
        assert_eq!(decode_op(instr), Op::Jmp);
        assert_eq!(decode_a(instr), 0);
        assert_eq!(decode_sbx(instr), 50);

        // Jmp -30
        let instr = encode_sbx(Op::Jmp, -30);
        assert_eq!(decode_op(instr), Op::Jmp);
        assert_eq!(decode_sbx(instr), -30);
    }

    #[test]
    fn all_opcodes_encodable() {
        // Verify every opcode survives a round-trip through the 6-bit field.
        let all = [
            Op::Move, Op::LoadNil, Op::LoadTrue, Op::LoadFalse, Op::LoadConst,
            Op::NewTable, Op::GetTable, Op::SetTable, Op::GetField, Op::SetField,
            Op::GetIndex, Op::SetIndex, Op::GetGlobal, Op::SetGlobal,
            Op::GetUpval, Op::SetUpval,
            Op::Add, Op::Sub, Op::Mul, Op::Div, Op::Mod, Op::Unm,
            Op::Concat, Op::Len,
            Op::Eq, Op::Ne, Op::Lt, Op::Le, Op::Gt, Op::Ge,
            Op::Not, Op::TestSet,
            Op::Jmp, Op::JmpIf, Op::JmpIfNot,
            Op::Call, Op::Return,
            Op::ForPrep, Op::ForLoop,
            Op::Closure,
            Op::Close, Op::Nop,
        ];
        for op in all {
            let instr = encode_abc(op, 0, 0, 0);
            assert_eq!(decode_op(instr), op, "round-trip failed for {op:?}");
        }
    }

    #[test]
    fn opcode_count_fits_6_bits() {
        assert!(
            OP_COUNT <= (1 << OPCODE_BITS) as u8,
            "too many opcodes ({OP_COUNT}) for {OPCODE_BITS}-bit field",
        );
    }

    #[test]
    #[should_panic(expected = "invalid opcode")]
    fn decode_invalid_opcode_panics() {
        // Craft an instruction with an out-of-range opcode.
        let bad = OP_COUNT as u32;
        decode_op(bad);
    }

    #[test]
    fn instruction_word_is_32_bits() {
        // Verify no bits are lost: set every field to its maximum value.
        let instr = encode_abc(Op::Nop, 255, 511, 511);
        assert_eq!(instr & !0u32, instr, "instruction overflows u32");
    }

    #[test]
    fn b_c_fields_independent() {
        // Ensure B and C do not alias each other.
        let instr_b = encode_abc(Op::Add, 0, 511, 0);
        assert_eq!(decode_b(instr_b), 511);
        assert_eq!(decode_c(instr_b), 0);

        let instr_c = encode_abc(Op::Add, 0, 0, 511);
        assert_eq!(decode_b(instr_c), 0);
        assert_eq!(decode_c(instr_c), 511);
    }
}
