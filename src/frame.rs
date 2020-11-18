#[cfg(test)]
mod tests;

use core::cmp::Ordering;
use core::ops::{Deref, DerefMut};

use crate::{Id, IdReg};

/// A CAN data or remote frame.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frame {
    pub(crate) id: IdReg,
    pub(crate) data: Data,
}

impl Frame {
    /// Creates a new data frame.
    pub fn new_data(id: Id, data: Data) -> Self {
        let id = match id {
            Id::Standard(id) => IdReg::new_standard(id),
            Id::Extended(id) => IdReg::new_extended(id),
        };

        Self { id, data }
    }

    /// Creates a new remote frame with configurable data length code (DLC).
    pub fn new_remote(id: Id, dlc: u8) -> Result<Frame, ()> {
        if dlc >= 8 {
            return Err(());
        }

        let mut frame = Self::new_data(id, Data::empty());
        // Just extend the data length, even with no data present. The API does not hand out this
        // `Data` object.
        frame.data.len = dlc;
        frame.id = frame.id.with_rtr(true);
        Ok(frame)
    }

    /// Returns true if this frame is an extended frame.
    pub fn is_extended(&self) -> bool {
        self.id.is_extended()
    }

    /// Returns true if this frame is a standard frame.
    pub fn is_standard(&self) -> bool {
        self.id.is_standard()
    }

    /// Returns true if this frame is a remote frame.
    pub fn is_remote_frame(&self) -> bool {
        self.id.rtr()
    }

    /// Returns true if this frame is a data frame.
    pub fn is_data_frame(&self) -> bool {
        !self.is_remote_frame()
    }

    /// Returns the frame identifier.
    pub fn id(&self) -> Id {
        self.id.to_id()
    }

    /// Returns the priority of this frame.
    pub fn priority(&self) -> FramePriority {
        FramePriority(self.id)
    }

    /// Returns the data length code (DLC) which is in the range 0..8.
    ///
    /// For data frames the DLC value always matches the length of the data.
    /// Remote frames do not carry any data, yet the DLC can be greater than 0.
    pub fn dlc(&self) -> usize {
        self.data.len()
    }

    /// Returns the frame data (0..8 bytes in length) if this is a data frame.
    ///
    /// If this is a remote frame, returns `None`.
    pub fn data(&self) -> Option<&Data> {
        if self.is_data_frame() {
            Some(&self.data)
        } else {
            None
        }
    }
}

/// Priority of a CAN frame.
///
/// The priority of a frame is determined by the bits that are part of the *arbitration field*.
/// These consist of the frame identifier bits (including the *IDE* bit, which is 0 for extended
/// frames and 1 for standard frames), as well as the *RTR* bit, which determines whether a frame
/// is a data or remote frame. Lower values of the *arbitration field* have higher priority.
///
/// This struct wraps the *arbitration field* and implements `PartialOrd` and `Ord` accordingly,
/// ordering higher priorities greater than lower ones.
#[derive(Debug, Copy, Clone)]
pub struct FramePriority(IdReg);

/// Ordering is based on the Identifier and frame type (data vs. remote) and can be used to sort
/// frames by priority.
impl Ord for FramePriority {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for FramePriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for FramePriority {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for FramePriority {}

/// Payload of a CAN data frame.
///
/// Contains 0 to 8 Bytes of data.
///
/// `Data` implements `From<[u8; N]>` for all `N` up to 8, which provides a convenient lossless
/// conversion from fixed-length arrays.
#[derive(Debug, Copy, Clone)]
pub struct Data {
    pub(crate) len: u8,
    pub(crate) bytes: [u8; 8],
}

impl Data {
    /// Creates a data payload from a raw byte slice.
    ///
    /// Returns `None` if `data` contains more than 8 Bytes (which is the maximum).
    ///
    /// `Data` can also be constructed from fixed-length arrays up to length 8 via `From`/`Into`.
    pub fn new(data: &[u8]) -> Option<Self> {
        if data.len() > 8 {
            return None;
        }

        let mut bytes = [0; 8];
        bytes[..data.len()].copy_from_slice(data);

        Some(Self {
            len: data.len() as u8,
            bytes,
        })
    }

    /// Creates an empty data payload containing 0 bytes.
    #[inline]
    pub const fn empty() -> Self {
        Self {
            len: 0,
            bytes: [0; 8],
        }
    }

    /// Returns the numeber of bytes in the data payload.
    #[inline]
    pub fn len(&self) -> usize {
        self.len.into()
    }

    /// Returns `true` when this `Data` is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Deref for Data {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &[u8] {
        &self.bytes[..self.len()]
    }
}

impl DerefMut for Data {
    #[inline]
    fn deref_mut(&mut self) -> &mut [u8] {
        let len = self.len();
        &mut self.bytes[..len]
    }
}

impl AsRef<[u8]> for Data {
    fn as_ref(&self) -> &[u8] {
        self.deref()
    }
}

impl AsMut<[u8]> for Data {
    fn as_mut(&mut self) -> &mut [u8] {
        self.deref_mut()
    }
}

impl PartialEq for Data {
    fn eq(&self, other: &Self) -> bool {
        self.as_ref() == other.as_ref()
    }
}

impl Eq for Data {}

macro_rules! data_from_array {
    ( $($len:literal),+ ) => {
        $(
            impl From<[u8; $len]> for Data {
                #[inline]
                fn from(arr: [u8; $len]) -> Self {
                    let mut bytes = [0; 8];
                    bytes[..$len].copy_from_slice(&arr);
                    Self {
                        len: $len,
                        bytes,
                    }
                }
            }
        )+
    };
}

data_from_array!(0, 1, 2, 3, 4, 5, 6, 7, 8);
