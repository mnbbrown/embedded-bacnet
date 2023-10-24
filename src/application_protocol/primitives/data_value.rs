use core::{fmt::Display, str::from_utf8};

use flagset::{FlagSet, Flags};

use crate::common::{
    daily_schedule::{WeeklySchedule, WeeklyScheduleWrite},
    error::Error,
    helper::{decode_unsigned, encode_unsigned, get_len_u32},
    io::{Reader, Writer},
    object_id::{ObjectId, ObjectType},
    property_id::PropertyId,
    spec::{
        Binary, EngineeringUnits, EventState, LogBufferResultFlags, LoggingType, NotifyType,
        StatusFlags,
    },
    tag::{ApplicationTagNumber, Tag, TagNumber},
};

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ApplicationDataValue<'a> {
    Boolean(bool),
    Real(f32),
    Double(f64),
    Date(Date),
    Time(Time),
    ObjectId(ObjectId),
    CharacterString(CharacterString<'a>),
    Enumerated(Enumerated),
    BitString(BitString<'a>),
    UnsignedInt(u32),
    WeeklySchedule(WeeklySchedule<'a>),
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ApplicationDataValueWrite<'a> {
    Boolean(bool),
    Enumerated(Enumerated),
    Real(f32),
    WeeklySchedule(WeeklyScheduleWrite<'a>),
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Enumerated {
    Units(EngineeringUnits),
    Binary(Binary),
    ObjectType(ObjectType),
    EventState(EventState),
    NotifyType(NotifyType),
    LoggingType(LoggingType),
    Unknown(u32),
}

impl Enumerated {
    pub fn encode(&self, writer: &mut Writer) {
        let value = match self {
            Self::Units(x) => *x as u32,
            Self::Binary(x) => *x as u32,
            Self::ObjectType(x) => *x as u32,
            Self::EventState(x) => *x as u32,
            Self::NotifyType(x) => *x as u32,
            Self::LoggingType(x) => *x as u32,
            Self::Unknown(x) => *x,
        };
        let len = get_len_u32(value);
        let tag = Tag::new(
            TagNumber::Application(ApplicationTagNumber::Enumerated),
            len,
        );
        tag.encode(writer);
        encode_unsigned(writer, len, value as u64);
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Date {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub wday: u8, // 1 (Monday) to 7 (Sunday)
}

impl Date {
    pub const LEN: u32 = 4; // 4 bytes

    //  year = years since 1900, wildcard=1900+255
    //  month 1=Jan
    //  day = day of month
    //  wday 1=Monday...7=Sunday
    pub fn decode_from_tag(tag: &Tag) -> Self {
        let value = tag.value;
        let value = value.to_be_bytes();
        Self::decode_inner(value)
    }

    pub fn decode(reader: &mut Reader, buf: &[u8]) -> Self {
        let value = reader.read_bytes(buf);
        Self::decode_inner(value)
    }

    fn decode_inner(value: [u8; 4]) -> Self {
        let year = value[0] as u16 + 1900;
        let month = value[1];
        let day = value[2];
        let wday = value[3];
        Self {
            year,
            month,
            day,
            wday,
        }
    }

    pub fn encode(&self, writer: &mut Writer) {
        let year = (self.year - 1900) as u8;
        writer.push(year);
        writer.push(self.month);
        writer.push(self.day);
        writer.push(self.wday);
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Time {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub hundredths: u8,
}

impl Time {
    pub const LEN: u32 = 4; // 4 bytes

    // assuming that this comes from a Time tag
    pub fn decode(reader: &mut Reader, buf: &[u8]) -> Self {
        let hour = reader.read_byte(buf);
        let minute = reader.read_byte(buf);
        let second = reader.read_byte(buf);
        let hundredths = reader.read_byte(buf);
        Time {
            hour,
            minute,
            second,
            hundredths,
        }
    }

    pub fn encode(&self, writer: &mut Writer) {
        writer.push(self.hour);
        writer.push(self.minute);
        writer.push(self.second);
        writer.push(self.hundredths);
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CharacterString<'a> {
    pub inner: &'a str,
}

impl<'a> Display for ApplicationDataValue<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ApplicationDataValue::Real(x) => write!(f, "{}", x),
            ApplicationDataValue::Double(x) => write!(f, "{}", x),
            ApplicationDataValue::CharacterString(x) => write!(f, "{}", &x.inner),
            ApplicationDataValue::Boolean(x) => write!(f, "{}", x),
            x => write!(f, "{:?}", x),
        }
    }
}

#[derive(Debug)]
pub enum BitString<'a> {
    StatusFlags(FlagSet<StatusFlags>),
    LogBufferResultFlags(FlagSet<LogBufferResultFlags>),
    Custom(CustomBitStream<'a>),
}

#[cfg(feature = "defmt")]
impl<'a> defmt::Format for BitString<'a> {
    fn format(&self, _fmt: defmt::Formatter) {
        // do nothing for now because it is too complicated due to StatusFlags
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CustomBitStream<'a> {
    pub unused_bits: u8,
    pub bits: &'a [u8],
}

impl<'a> BitString<'a> {
    pub fn encode(&self, writer: &mut Writer) {
        match self {
            Self::StatusFlags(x) => {
                Tag::new(TagNumber::Application(ApplicationTagNumber::BitString), 2).encode(writer);
                writer.push(0); // no unused bits
                writer.push(x.bits());
            }
            Self::LogBufferResultFlags(x) => {
                Tag::new(TagNumber::Application(ApplicationTagNumber::BitString), 2).encode(writer);
                writer.push(0); // no unused bits
                writer.push(x.bits());
            }
            Self::Custom(x) => {
                Tag::new(
                    TagNumber::Application(ApplicationTagNumber::BitString),
                    x.bits.len() as u32 + 1,
                )
                .encode(writer);
                writer.push(0); // no unused bits
                writer.extend_from_slice(x.bits);
            }
        }
    }

    pub fn decode(
        property_id: PropertyId,
        len: u32,
        reader: &mut Reader,
        buf: &'a [u8],
    ) -> Result<Self, Error> {
        let unused_bits = reader.read_byte(buf);
        match property_id {
            PropertyId::PropStatusFlags => {
                let status_flags = Self::decode_byte_flag(reader.read_byte(buf))?;
                Ok(Self::StatusFlags(status_flags))
            }
            PropertyId::PropLogBuffer => {
                let flags = Self::decode_byte_flag(reader.read_byte(buf))?;
                Ok(Self::LogBufferResultFlags(flags))
            }
            _ => {
                let len = (len - 1) as usize; // we have already read a byte
                let bits = reader.read_slice(len as usize, buf);
                Ok(Self::Custom(CustomBitStream { unused_bits, bits }))
            }
        }
    }

    fn decode_byte_flag<T: Flags>(byte: T::Type) -> Result<FlagSet<T>, Error> {
        match FlagSet::new(byte) {
            Ok(x) => Ok(x),
            Err(_) => Err(Error::InvalidValue("invalid flag bitstream")),
        }
    }
}

impl<'a> CharacterString<'a> {
    pub fn decode(len: u32, reader: &mut Reader, buf: &'a [u8]) -> Self {
        let character_set = reader.read_byte(buf);
        if character_set != 0 {
            unimplemented!("non-utf8 characterset not supported")
        }
        let slice = reader.read_slice(len as usize - 1, buf);
        CharacterString {
            inner: from_utf8(slice).unwrap(),
        }
    }
}

impl<'a> ApplicationDataValueWrite<'a> {
    pub fn encode(&self, writer: &mut Writer) {
        match self {
            Self::Boolean(x) => {
                let len = 1;
                let tag = Tag::new(TagNumber::Application(ApplicationTagNumber::Boolean), len);
                tag.encode(writer);
                let value = if *x { 1_u8 } else { 0_u8 };
                writer.push(value)
            }
            Self::Real(x) => {
                let len = 4;
                let tag = Tag::new(TagNumber::Application(ApplicationTagNumber::Real), len);
                tag.encode(writer);
                writer.extend_from_slice(&f32::to_be_bytes(*x))
            }
            Self::Enumerated(x) => {
                x.encode(writer);
            }
            Self::WeeklySchedule(x) => x.encode(writer),
        }
    }
}

impl<'a> ApplicationDataValue<'a> {
    pub fn encode(&self, writer: &mut Writer) {
        match self {
            ApplicationDataValue::Boolean(x) => Tag::new(
                TagNumber::Application(ApplicationTagNumber::Boolean),
                if *x { 1 } else { 0 },
            )
            .encode(writer),
            ApplicationDataValue::Real(x) => {
                Tag::new(TagNumber::Application(ApplicationTagNumber::Real), 4).encode(writer);
                writer.extend_from_slice(&x.to_be_bytes());
            }
            ApplicationDataValue::Date(x) => {
                Tag::new(
                    TagNumber::Application(ApplicationTagNumber::Date),
                    Date::LEN,
                )
                .encode(writer);
                x.encode(writer);
            }
            ApplicationDataValue::Time(x) => {
                Tag::new(
                    TagNumber::Application(ApplicationTagNumber::Time),
                    Time::LEN,
                )
                .encode(writer);
                x.encode(writer);
            }
            ApplicationDataValue::ObjectId(x) => {
                Tag::new(
                    TagNumber::Application(ApplicationTagNumber::ObjectId),
                    ObjectId::LEN,
                )
                .encode(writer);
                x.encode(writer);
            }
            ApplicationDataValue::CharacterString(x) => {
                let utf8_encoded = x.inner.as_bytes(); // strings in rust are utf8 encoded already
                Tag::new(
                    TagNumber::Application(ApplicationTagNumber::CharacterString),
                    utf8_encoded.len() as u32 + 1, // keep space for encoding byte
                )
                .encode(writer);
                writer.push(0); // utf8 encoding
                writer.extend_from_slice(utf8_encoded);
            }
            ApplicationDataValue::Enumerated(x) => {
                x.encode(writer);
            }
            ApplicationDataValue::BitString(x) => {
                x.encode(writer);
            }
            ApplicationDataValue::UnsignedInt(x) => {
                Tag::new(TagNumber::Application(ApplicationTagNumber::UnsignedInt), 4)
                    .encode(writer);
                writer.extend_from_slice(&x.to_be_bytes());
            }
            ApplicationDataValue::WeeklySchedule(x) => {
                todo!("{:?}", x);
            }

            x => todo!("{:?}", x),
        };
    }

    pub fn decode(
        tag: &Tag,
        object_id: &ObjectId,
        property_id: &PropertyId,
        reader: &mut Reader,
        buf: &'a [u8],
    ) -> Self {
        let tag_num = match &tag.number {
            TagNumber::Application(x) => x,
            unknown => panic!("application tag number expected: {:?}", unknown),
        };

        match tag_num {
            ApplicationTagNumber::Real => {
                assert_eq!(tag.value, 4, "read tag should have length of 4");
                ApplicationDataValue::Real(f32::from_be_bytes(reader.read_bytes(buf)))
            }
            ApplicationTagNumber::ObjectId => {
                let object_id = ObjectId::decode(tag.value, reader, buf).unwrap();
                ApplicationDataValue::ObjectId(object_id)
            }
            ApplicationTagNumber::CharacterString => {
                let text = CharacterString::decode(tag.value, reader, buf);
                ApplicationDataValue::CharacterString(text)
            }
            ApplicationTagNumber::Enumerated => {
                let value = decode_unsigned(tag.value, reader, buf) as u32;
                let value = match property_id {
                    PropertyId::PropUnits => {
                        let units = value.try_into().unwrap();
                        Enumerated::Units(units)
                    }
                    PropertyId::PropPresentValue => match object_id.object_type {
                        ObjectType::ObjectBinaryInput
                        | ObjectType::ObjectBinaryOutput
                        | ObjectType::ObjectBinaryValue => {
                            let binary = value.try_into().unwrap();
                            Enumerated::Binary(binary)
                        }
                        _ => Enumerated::Unknown(value),
                    },
                    PropertyId::PropObjectType => {
                        let object_type = ObjectType::try_from(value).unwrap();
                        Enumerated::ObjectType(object_type)
                    }
                    PropertyId::PropEventState => {
                        let event_state = EventState::try_from(value).unwrap();
                        Enumerated::EventState(event_state)
                    }
                    PropertyId::PropNotifyType => {
                        let notify_type = NotifyType::try_from(value).unwrap();
                        Enumerated::NotifyType(notify_type)
                    }
                    PropertyId::PropLoggingType => {
                        let logging_type = LoggingType::try_from(value).unwrap();
                        Enumerated::LoggingType(logging_type)
                    }

                    _ => Enumerated::Unknown(value),
                };
                ApplicationDataValue::Enumerated(value)
            }
            ApplicationTagNumber::BitString => {
                let bit_string = BitString::decode(*property_id, tag.value, reader, buf).unwrap();
                ApplicationDataValue::BitString(bit_string)
            }
            ApplicationTagNumber::Boolean => {
                let value = tag.value > 0;
                ApplicationDataValue::Boolean(value)
            }
            ApplicationTagNumber::UnsignedInt => {
                let value = decode_unsigned(tag.value, reader, buf) as u32;
                ApplicationDataValue::UnsignedInt(value)
            }
            ApplicationTagNumber::Time => {
                assert_eq!(tag.value, 4); // 4 bytes
                let time = Time::decode(reader, buf);
                ApplicationDataValue::Time(time)
            }
            ApplicationTagNumber::Date => {
                // let date = Date::decode_from_tag(&tag);
                let date = Date::decode(reader, buf);
                ApplicationDataValue::Date(date)
            }

            x => unimplemented!("{:?}", x),
        }
    }
}
