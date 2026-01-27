use crate::{error::Result, pic};

pub const DETOUR_RANGE: usize = 0x800_0000; // 128 MB

pub fn prolog_margin(_target: *const ()) -> usize {
  16
}

pub fn relay_builder(target: *const (), detour: *const ()) -> Result<Option<pic::CodeEmitter>> {
  use super::thunk;

  let displacement = (target as isize).wrapping_sub(detour as isize);

  // If the displacement is outside the ±128MB range, we need a relay
  if !crate::arch::is_within_range(displacement) {
    let mut emitter = pic::CodeEmitter::new();
    emitter.add_thunk(thunk::jmp_abs(detour as usize));
    Ok(Some(emitter))
  } else {
    Ok(None)
  }
}
