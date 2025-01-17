//! TRB (Transfer Request Block).

use bit_field::BitField;
use core::convert::TryInto;
use num_derive::FromPrimitive;

macro_rules! reserved{
    ($name:ident($ty:expr) {
        $([$index:expr] $range:expr);*
    })=>{
        impl TryFrom<[u32;4]> for $name{
            type Error=[u32;4];

            fn try_from(raw:[u32;4])->Result<Self,Self::Error>{
                use crate::ring::trb::Type;

                $(if raw[$index].get_bits($range) != 0{
                    return Err(raw);
                })*

                if raw[3].get_bits(10..=15)!=$ty as _ {
                    return Err(raw);
                }

                Ok(Self(raw))
            }
        }
    }
}
macro_rules! add_trb {
    ($name:ident,$full:expr,$ty:expr) => {
        #[doc = $full ]
        #[repr(transparent)]
        #[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
        pub struct $name([u32; 4]);
        impl $name {
            /// Returns the wrapped array.
            #[must_use]
            pub fn into_raw(self) -> [u32; 4] {
                self.0
            }

            /// Returns the value of the Cycle Bit.
            #[must_use]
            pub fn cycle_bit(&self) -> bool {
                self.0[3].get_bit(0)
            }

            /// Sets the value of the Cycle Bit.
            pub fn set_cycle_bit(&mut self, b: bool) -> &mut Self {
                use bit_field::BitField;
                self.0[3].set_bit(0, b);
                self
            }

            fn set_trb_type(&mut self) -> &mut Self {
                use crate::ring::trb::Type;
                use bit_field::BitField;
                self.0[3].set_bits(10..=15, $ty as _);
                self
            }
        }
        impl AsRef<[u32]> for $name {
            fn as_ref(&self) -> &[u32] {
                &self.0
            }
        }
        impl From<$name> for [u32; 4] {
            fn from(t: $name) -> Self {
                t.0
            }
        }
    };
}
macro_rules! impl_default_simply_adds_trb_id {
    ($name:ident,$full:expr) => {
        impl $name{
            paste::paste! {
                #[doc = "Creates a new " $full ".\n\nThis method sets the sets the value of the TRB Type field properly. All the other fieldds are set to 0."]
                #[must_use]
                pub fn new()->Self{
                    *Self([0;4]).set_trb_type()
                }
            }
        }
        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}
macro_rules! add_trb_with_default {
    ($name:ident,$full:expr,$type:expr) => {
        add_trb!($name, $full, $type);
        impl_default_simply_adds_trb_id!($name, $full);
    };
}
macro_rules! impl_debug_for_trb{
    ($name:ident {
        $($method:ident),*
    })=>{
        impl core::fmt::Debug for $name{
            fn fmt(&self, f:&mut core::fmt::Formatter<'_>)->core::fmt::Result{
                f.debug_struct(core::stringify!($name))
                    $(.field(core::stringify!($method), &self.$method()))*
                    .field("cycle_bit", &self.cycle_bit())
                    .finish()
            }
        }
    }
}

macro_rules! allowed {
    (
        $(#[$outer:meta])*
        enum {
            $($(#[$doc:meta])* $variant:ident),+
        }
    ) => {
        $(#[$outer])*
            #[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
            pub enum Allowed {
                $($(#[$doc])* $variant($variant)),+
            }
        impl Allowed{
            /// Sets the value of the Cycle Bit.
            pub fn set_cycle_bit(&mut self,b:bool)->&mut Self{
                match self{
                    $(
                        Self::$variant(ref mut v) => {
                            v.set_cycle_bit(b);
                        }
                    ),+
                }
                self
            }

            /// Returns the value of the Cycle Bit.
            #[must_use]
            pub fn cycle_bit(&self)->bool{
                match self{
                    $( Self::$variant(ref v) => v.cycle_bit() ),+
                }
            }

            /// Returns the wrapped array.
            #[must_use]
            pub fn into_raw(self)->[u32;4]{
                match self{
                    $( Self::$variant(v) => v.into_raw() ),+
                }
            }
        }
        impl AsRef<[u32]> for Allowed {
            fn as_ref(&self) -> &[u32]{
                match self{
                    $( Self::$variant(ref v) => v.as_ref() ),+
                }
            }
        }
        $(
            impl From<$variant> for Allowed{
                fn from(v:$variant)->Self{
                    Self::$variant(v)
                }
            }
        )+
    };
}

pub mod command;
pub mod event;
pub mod transfer;

/// The bytes of a TRB.
pub const BYTES: usize = 16;

add_trb_with_default!(Link, "Link TRB", Type::Link);
impl Link {
    /// Sets the value of the Ring Segment Pointer field.
    ///
    /// # Panics
    ///
    /// This method panics if `p` is not 16-byte aligned.
    pub fn set_ring_segment_pointer(&mut self, p: u64) -> &mut Self {
        assert_eq!(
            p % 16,
            0,
            "The Ring Segment Pointer must be 16-byte aligned."
        );

        let l = p.get_bits(0..32);
        let u = p.get_bits(32..64);

        self.0[0] = l.try_into().unwrap();
        self.0[1] = u.try_into().unwrap();
        self
    }

    /// Returns the value of the Ring Segment Pointer field.
    #[must_use]
    pub fn ring_segment_pointer(&self) -> u64 {
        let l: u64 = self.0[0].into();
        let u: u64 = self.0[1].into();

        (u << 32) | l
    }

    /// Sets the value of the Interrupter Target field.
    pub fn set_interrupter_target(&mut self, t: u32) -> &mut Self {
        self.0[2].set_bits(22..=31, t);
        self
    }

    /// Returns the value of the Interrupter Target field.
    #[must_use]
    pub fn interrupter_target(&self) -> u32 {
        self.0[2].get_bits(22..=31)
    }

    /// Sets the value of the Toggle Cycle field.
    pub fn set_toggle_cycle(&mut self, c: bool) -> &mut Self {
        self.0[3].set_bit(1, c);
        self
    }

    /// Returns the value of the Toggle Cycle field.
    #[must_use]
    pub fn toggle_cycle(&self) -> bool {
        self.0[3].get_bit(1)
    }

    /// Sets the value of the Chain bit field.
    pub fn set_chain_bit(&mut self, b: bool) -> &mut Self {
        self.0[3].set_bit(4, b);
        self
    }

    /// Returns the value of the Chain bit field.
    #[must_use]
    pub fn chain_bit(&self) -> bool {
        self.0[3].get_bit(4)
    }

    /// Sets the value of the Interrupt On Completion field.
    pub fn set_interrupt_on_completion(&mut self, ioc: bool) -> &mut Self {
        self.0[3].set_bit(5, ioc);
        self
    }

    /// Returns the value of the Interrupt On Completion field.
    #[must_use]
    pub fn interrupt_on_completion(&self) -> bool {
        self.0[3].get_bit(5)
    }
}
impl_debug_for_trb!(Link {
    ring_segment_pointer,
    interrupter_target,
    toggle_cycle,
    chain_bit,
    interrupt_on_completion
});

/// TRB Type.
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash, FromPrimitive)]
pub enum Type {
    /// Normal TRB, 1
    Normal = 1,
    /// Setup Stage TRB, 2
    SetupStage = 2,
    /// Data Stage TRB, 3
    DataStage = 3,
    /// Status Stage TRB, 4
    StatusStage = 4,
    /// Isoch TRB, 5
    Isoch = 5,
    /// Link TRB, 6
    Link = 6,
    /// Event Data TRB, 7
    EventData = 7,
    /// No Op TRB (Transfer), 8
    NoopTransfer = 8,
    /// Enable Slot Command TRB, 9
    EnableSlot = 9,
    /// Disable Slot Command TRB, 10
    DisableSlot = 10,
    /// Address Device Command TRB, 11
    AddressDevice = 11,
    /// Configure Endpoint Command TRB, 12
    ConfigureEndpoint = 12,
    /// Evaluate Context Command TRB, 13
    EvaluateContext = 13,
    /// Reset Endpoint Command TRB, 14
    ResetEndpoint = 14,
    /// Stop Endpoint Command TRB, 15
    StopEndpoint = 15,
    /// Set TR Dequeue Pointer Command TRB, 16
    SetTrDequeuePointer = 16,
    /// Reset Device Command TRB, 17
    ResetDevice = 17,
    /// Force Event Command TRB, 18
    ForceEvent = 18,
    /// Negotiate Bandwidth Command TRB, 19
    NegotiateBandwidth = 19,
    /// Set Latency Tolerance Value Command TRB, 20
    SetLatencyToleranceValue = 20,
    /// Get Port Bandwidth Command TRB, 21
    GetPortBandwidth = 21,
    /// Force Header Command TRB, 22
    ForceHeader = 22,
    /// No Op Command TRB, 23
    NoopCommand = 23,
    /// Get Extended Property Command TRB, 24
    GetExtendedProperty = 24,
    /// Set Extended Property Command TRB, 25
    SetExtendedProperty = 25,
    /// Transfer Event TRB, 32
    TransferEvent = 32,
    /// Command Completion Event TRB, 33
    CommandCompletion = 33,
    /// Port Status Change Event TRB, 34
    PortStatusChange = 34,
    /// Bandwidth Request Event TRB, 35
    BandwidthRequest = 35,
    /// Doorbell Event TRB, 36
    Doorbell = 36,
    /// Host Controller Event TRB, 37
    HostController = 37,
    /// Device Notification Event TRB, 38
    DeviceNotification = 38,
    /// MFINDEX Wrap Event TRB, 39
    MfindexWrap = 39,
}
