//! This module contains structures representing a constant pool and its entries.
/*
 *     This file is part of Coffer.
 *
 *     Coffer is free software: you can redistribute it and/or modify
 *     it under the terms of the GNU Lesser General Public License as published by
 *     the Free Software Foundation, either version 3 of the License, or
 *     (at your option) any later version.
 *
 *     Coffer is distributed in the hope that it will be useful,
 *     but WITHOUT ANY WARRANTY; without even the implied warranty of
 *     MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *     GNU General Public License for more details.
 *
 *     You should have received a copy of the GNU Lesser General Public License
 *     along with Coffer. (LICENSE.md)  If not, see <https://www.gnu.org/licenses/>.
 */

use std::borrow::Cow;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::prelude::{BootstrapMethod, LazyBsm, Read, Result, Write};
use crate::{mod_utf8, ConstantPoolReader, ConstantPoolWriter, Error, ReadWrite};

#[derive(Clone, Debug, Copy)]
pub struct StrRef<'a>(pub &'a str);

impl<'a> ReadWrite for StrRef<'a> {
    fn read_from<T: Read>(_reader: &mut T) -> Result<Self> {
        Err(crate::Error::Invalid(
            "call",
            "read_from for StrRef is unimplemented".into(),
        ))
    }

    fn write_to<T: Write>(&self, writer: &mut T) -> Result<()> {
        writer.write_all(&mod_utf8::string_to_modified_utf8(self.0))?;
        Ok(())
    }
}

#[derive(ReadWrite, Debug, Clone, Copy)]
#[tag_type(u8)]
pub enum ConstEntryRef<'a> {
    UTF8(StrRef<'a>),
    #[tag(3)]
    Int(i32),
    Float(f32),
    Long(i64),
    Double(f64),
    Class(u16),
    String(u16),
    Field(u16, u16),
    Method(u16, u16),
    InterfaceMethod(u16, u16),
    NameAndType(u16, u16),
    #[tag(15)]
    MethodHandle(u8, u16),
    MethodType(u16),
    Dynamic(u16, u16),
    InvokeDynamic(u16, u16),
    Module(u16),
    Package(u16),
}

macro_rules! total_float {
    ($Name:ident($FloatTy:ty)) => {
        #[derive(Clone, Copy)]
        pub struct $Name(pub $FloatTy);
        impl PartialEq for $Name {
            fn eq(&self, other: &Self) -> bool {
                self.0.to_bits() == other.0.to_bits()
            }
        }
        impl Eq for $Name {}
        impl Hash for $Name {
            fn hash<H: Hasher>(&self, state: &mut H) {
                self.0.to_bits().hash(state)
            }
        }
        impl ReadWrite for $Name {
            fn read_from<T: Read>(reader: &mut T) -> Result<Self> {
                <$FloatTy>::read_from(reader).map(Self)
            }
            fn write_to<T: Write>(&self, writer: &mut T) -> Result<()> {
                self.0.write_to(writer)
            }
        }
        impl fmt::Debug for $Name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
        impl From<$FloatTy> for $Name {
            fn from(x: $FloatTy) -> $Name {
                $Name(x)
            }
        }
    };
}

total_float!(TotalF32(f32));
total_float!(TotalF64(f64));

/// A raw constant entry that has unresolved indices to other entries.
#[derive(ReadWrite, Debug, Clone, Hash, PartialEq, Eq)]
#[tag_type(u8)]
pub enum RawConstantEntry {
    #[tag(1)]
    UTF8(Cow<'static, str>),
    #[tag(3)]
    Int(i32),
    Float(TotalF32),
    Long(i64),
    Double(TotalF64),
    Class(u16),
    String(u16),
    Field(u16, u16),
    Method(u16, u16),
    InterfaceMethod(u16, u16),
    NameAndType(u16, u16),
    #[tag(15)]
    MethodHandle(u8, u16),
    MethodType(u16),
    Dynamic(u16, u16),
    InvokeDynamic(u16, u16),
    Module(u16),
    Package(u16),
}

impl RawConstantEntry {
    /// returns the size that this entry takes.
    #[inline]
    pub const fn size(&self) -> u16 {
        match self {
            RawConstantEntry::Long(_) | RawConstantEntry::Double(_) => 2,
            _ => 1,
        }
    }
    /// Returns `true` if this entry is a Long/Double constant, which takes 2 indices.
    #[inline]
    pub const fn is_wide(&self) -> bool {
        matches!(
            self,
            RawConstantEntry::Long(_) | RawConstantEntry::Double(_)
        )
    }
}

/// A simple constant pool reader implementation using hashmaps for constant entries and bootstrap method references.
#[derive(Debug)]
pub struct MapCp {
    /// The entries of this constant pool, represented as a hashmap
    /// as some entries may be absent when they are preceded by a double/long entry
    pub entries: HashMap<u16, RawConstantEntry>,
    refs: HashMap<u16, Vec<Rc<LazyBsm>>>,
}

/// A constant pool writer implementation using a vector and a number for tracking entries.
pub struct VecCp {
    entries: Vec<RawConstantEntry>,
    prev_entries: HashMap<RawConstantEntry, u16>,
    /// Not actual len. (if e.wide 2 else 1 for e in entries) + 1 in pseudocode
    len: u16,
    pub(crate) bsm: Vec<BootstrapMethod>,
}
impl VecCp {
    /// Creates an empty constant pool.
    #[inline]
    pub fn new() -> Self {
        Self {
            entries: vec![],
            prev_entries: HashMap::default(),
            len: 1,
            bsm: vec![],
        }
    }
}

impl Default for VecCp {
    fn default() -> Self {
        Self::new()
    }
}

impl MapCp {
    /// Creates a new constant pool with no entries.
    #[inline]
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            refs: HashMap::new(),
        }
    }
}

impl Default for MapCp {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl ReadWrite for MapCp {
    fn read_from<T: Read>(reader: &mut T) -> Result<Self> {
        let mut cp = MapCp::new();
        let count = u16::read_from(reader)?;
        let mut i = 1;
        while i < count {
            let entry = RawConstantEntry::read_from(reader)?;
            let idx = i;
            i += entry.size();
            cp.entries.insert(idx, entry);
        }
        Ok(cp)
    }

    fn write_to<T: Write>(&self, _writer: &mut T) -> Result<()> {
        unimplemented!()
    }
}

impl ConstantPoolReader for MapCp {
    fn read_raw(&mut self, idx: u16) -> Option<RawConstantEntry> {
        self.entries.get(&idx).cloned()
    }

    fn resolve_later(&mut self, bsm_idx: u16, bsm: Rc<LazyBsm>) {
        self.refs.entry(bsm_idx).or_default().push(bsm);
    }

    fn bootstrap_methods(&mut self, bsms: &[BootstrapMethod]) -> Result<()> {
        for (i, b) in bsms.iter().enumerate() {
            if let Entry::Occupied(bsm) = self.refs.entry(i as _) {
                for reg in bsm.remove() {
                    reg.fill(b.clone()).unwrap()
                }
            }
        }
        if let Some((_, v)) = self.refs.iter().find(|(_, v)| !v.is_empty()) {
            Err(Error::Invalid(
                "reference(s) to bootstrap method",
                Cow::from(format!("{:?}", v)),
            ))
        } else {
            Ok(())
        }
    }
}

impl ReadWrite for VecCp {
    fn read_from<T: Read>(_reader: &mut T) -> Result<Self> {
        unimplemented!()
    }

    fn write_to<T: Write>(&self, writer: &mut T) -> Result<()> {
        self.len.write_to(writer)?;
        for e in &self.entries {
            e.write_to(writer)?;
        }
        Ok(())
    }
}

impl ConstantPoolWriter for VecCp {
    fn insert_raw(&mut self, value: RawConstantEntry) -> u16 {
        if let Some(val) = self.prev_entries.get(&value) {
            return *val;
        }

        let idx = self.len;
        self.len = idx + value.size();
        self.entries.push(value.clone());
        self.prev_entries.insert(value, idx);

        idx
    }

    fn insert_bsm(&mut self, bsm: BootstrapMethod) -> u16 {
        let ret = self.bsm.len() as u16;
        self.bsm.push(bsm);
        ret
    }
}
