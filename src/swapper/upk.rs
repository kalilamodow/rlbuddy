use anyhow::{Context as _, Result, ensure};
use byteorder::{LittleEndian, ReadBytesExt as _, WriteBytesExt as _};
use std::{
    fs,
    io::{self, Cursor, Read, Seek, SeekFrom, Write},
    path::Path,
};

use crate::swapper::{encryption::RlAesKey, service::ItemId};

trait ReadBytes: Read {
    fn read_bytes(&mut self, n_bytes: usize) -> io::Result<Vec<u8>> {
        let mut buffer = vec![0u8; n_bytes];
        self.read_exact(&mut buffer)?;
        Ok(buffer)
    }

    fn read_bytes_const<const N_BYTES: usize>(&mut self) -> io::Result<[u8; N_BYTES]> {
        let mut buffer = [0u8; N_BYTES];
        self.read_exact(&mut buffer)?;
        Ok(buffer)
    }
}

impl<T: Read + ?Sized> ReadBytes for T {}

trait UPKPart: Sized {
    fn serialize(&self, writer: &mut impl Write, v33: bool) -> Result<()>;
    fn deserialize(reader: &mut impl Read, v33: bool) -> Result<Self>;
}

// i lowkey cant figure out how to make a #[derive] thing
macro_rules! fstruct {
    (
        $name:ident {
            $(
                $field:ident : $field_type:ty
            ),* $(,)?
        }
    ) => {
        #[derive(Debug, Clone)]
        struct $name {
            $(
                $field: $field_type,
            )*
        }

        impl UPKPart for $name {
            fn serialize(
                &self,
                writer: &mut impl Write,
                v33: bool,
            ) -> Result<()> {
                $(
                    self.$field.serialize(writer, v33).context(concat!(
                        stringify!($name),
                        ": serializing property ",
                        stringify!($field)
                    ))?;
                )*

                Ok(())
            }

            fn deserialize(
                reader: &mut impl Read,
                v33: bool,
            ) -> Result<Self> {
                Ok(Self {
                    $(
                        $field: <$field_type>::deserialize(reader, v33).context(concat!(
                            stringify!($name),
                            ": deserializing property ",
                            stringify!($field)
                        ))?,
                    )*
                })
            }
        }
    };
}

macro_rules! fbuiltin {
    ($type:ty, $readfn:ident, $writefn:ident) => {
        impl UPKPart for $type {
            fn serialize(&self, writer: &mut impl Write, _: bool) -> Result<()> {
                writer
                    .$writefn::<LittleEndian>(*self)
                    .context(concat!("serializing builtin type ", stringify!($type)))?;

                Ok(())
            }

            fn deserialize(reader: &mut impl Read, _: bool) -> Result<Self> {
                Ok(reader
                    .$readfn::<LittleEndian>()
                    .context(concat!("deserializing builtin type ", stringify!($type)))?)
            }
        }
    };
}

fbuiltin!(i16, read_i16, write_i16);
fbuiltin!(i32, read_i32, write_i32);
fbuiltin!(i64, read_i64, write_i64);
fbuiltin!(u16, read_u16, write_u16);
fbuiltin!(u32, read_u32, write_u32);
fbuiltin!(u64, read_u64, write_u64);

#[derive(Debug, Clone)]
struct TArray<T: UPKPart> {
    inner: Vec<T>,
}

impl<T: UPKPart> UPKPart for TArray<T> {
    fn serialize(&self, writer: &mut impl Write, v33: bool) -> Result<()> {
        writer
            .write_i32::<LittleEndian>(self.inner.len() as i32)
            .context("TArray: serializing length")?;

        for element in &self.inner {
            element
                .serialize(writer, v33)
                .context("TArray: serializing an element")?;
        }

        Ok(())
    }

    fn deserialize(reader: &mut impl Read, v33: bool) -> Result<Self> {
        let length = reader
            .read_i32::<LittleEndian>()
            .context("TArray: reading length")? as usize;
        let mut list = Vec::with_capacity(length);

        for _ in 0..length {
            let element =
                T::deserialize(reader, v33).context("TArray: deserializing an element")?;
            list.push(element);
        }

        Ok(Self { inner: list })
    }
}

fstruct!( FUnknownTypeInFPackageFileSummary {
    unknown_1: i32,
    unknown_2: i32,
    unknown_3: i32,
    unknown_4: i32,
    unknown_5: i32,
    unknown_6: TArray<i32>,
});

fstruct!(FGuid {
    a: u32,
    b: u32,
    c: u32,
    d: u32
});

fstruct!(FGenerationInfo {
    export_count: i32,
    name_count: i32,
    unknown_1: i32
});

fstruct!(FName {
    name_index: i32,
    instance_number: i32
});

fstruct!(FImportEntry {
    class_package: FName,
    class_name: FName,
    outer_index: i32,
    object_name: FName
});

fstruct!(FExportEntry {
    class_index: i32,
    super_index: i32,
    outer_index: i32,
    object_name: FName,
    archetype_index: i32,
    object_flags: u64,
    serial_size: i32,
    serial_offset: i64,
    export_flags: i32,
    net_objects: TArray<i32>,
    package_guid: FGuid,
    package_flags: i32,
});

fstruct!(FNameEntry {
    name: FString,
    flags: u64
});

#[derive(Debug, Clone)]
struct FCompressedChunkInfo {
    uncompressed_offset: i64,
    uncompressed_size: i32,
    compressed_offset: i64,
    compressed_size: i32,
    nonce: Option<[u8; 12]>,
}

impl UPKPart for FCompressedChunkInfo {
    fn serialize(&self, writer: &mut impl Write, v33: bool) -> Result<()> {
        self.uncompressed_offset.serialize(writer, v33)?;
        self.uncompressed_size.serialize(writer, v33)?;
        self.compressed_offset.serialize(writer, v33)?;
        self.compressed_size.serialize(writer, v33)?;
        if let Some(nonce) = &self.nonce {
            writer.write(nonce)?;
        }

        Ok(())
    }

    fn deserialize(reader: &mut impl Read, v33: bool) -> Result<Self> {
        Ok(Self {
            uncompressed_offset: i64::deserialize(reader, v33)?,
            uncompressed_size: i32::deserialize(reader, v33)?,
            compressed_offset: i64::deserialize(reader, v33)?,
            compressed_size: i32::deserialize(reader, v33)?,
            nonce: v33.then(|| reader.read_bytes_const::<12>()).transpose()?,
        })
    }
}

#[derive(Debug, Clone)]
struct FString {
    inner: String,
    is_unicode: bool,
}

impl UPKPart for FString {
    fn serialize(&self, writer: &mut impl Write, _: bool) -> Result<()> {
        if self.is_unicode {
            let utf16: Vec<_> = self.inner.encode_utf16().collect();
            writer
                .write_i32::<LittleEndian>(-i32::try_from(utf16.len() + 1)?)
                .context("FString: writing length")?;

            for word in utf16 {
                writer.write_u16::<LittleEndian>(word)?;
            }

            // null terminator
            writer.write_u16::<LittleEndian>(0)?;
        } else {
            let utf8 = self.inner.as_bytes();
            writer.write_i32::<LittleEndian>(i32::try_from(utf8.len() + 1)?)?;
            for byte in utf8 {
                writer.write_u8(*byte).unwrap();
            }
            writer.write_u8(0)?;
        }

        Ok(())
    }

    fn deserialize(reader: &mut impl Read, _: bool) -> Result<Self> {
        let length = reader
            .read_i32::<LittleEndian>()
            .context("FString: reading length")?;
        let is_unicode = length < 0;

        let decoded = if is_unicode {
            let n_words = -length as usize;

            let mut raw = Cursor::new(
                reader
                    .read_bytes(n_words * 2)
                    .context("FString: reading unicode bytes")?,
            );
            let mut utf16s = Vec::with_capacity(n_words);
            // - 1 for null terminator
            for _ in 0..(n_words - 1) {
                let word = raw.read_u16::<LittleEndian>()?;
                utf16s.push(word);
            }

            String::from_utf16(&utf16s).context("FString: decoding bytes to utf16")?
        } else {
            let n_letters = length as usize;
            let mut raw = reader
                .read_bytes(n_letters)
                .context("FString: reading utf8 bytes")?;
            raw.pop(); // null terminator
            String::from_utf8(raw).context("FString: decoding bytes to utf8")?
        };

        Ok(Self {
            inner: decoded,
            is_unicode,
        })
    }
}

fstruct!(FPackageFileSummary {
    tag: u32,
    file_version: u16,
    licensee_version: u16,
    total_header_size: i32,
    folder_name: FString,
    package_flags: u32,

    name_count: i32,
    name_offset: i32,
    export_count: i32,
    export_offset: i32,
    import_count: i32,
    import_offset: i32,
    depends_offset: i32,

    unknown_1: i32,
    unknown_2: i32,
    unknown_3: i32,
    unknown_4: i32,

    guid: FGuid,
    generations: TArray<FGenerationInfo>,

    engine_version: u32,
    cooker_version: u32,
    compression_flags: i32,

    // always length 0. its actually stored in the encrypted region
    compressed_chunks: TArray<FCompressedChunkInfo>,
    unknown_5: i32,

    unknown_6: TArray<FString>,
    unknown_7: TArray<FUnknownTypeInFPackageFileSummary>,

    garbage_size: i32,
    compressed_chunk_info_offset: i32,
    last_aes_block_size: i32,
});

const PACKAGE_FILE_MAGIC: u32 = 0x9E2A83C1;

impl FPackageFileSummary {
    pub fn v33(&self) -> bool {
        self.licensee_version >= 33
    }

    pub fn is_valid(&self) -> bool {
        self.tag == PACKAGE_FILE_MAGIC
    }

    pub fn encrypted_region_size(&self) -> i32 {
        let actual_size = self.total_header_size - self.garbage_size - self.name_offset;
        (actual_size + 15) & !15
    }
}

fn find_valid_aes_key<'a>(
    summary: &FPackageFileSummary,
    encrypted_tables_data: &[u8],
    keys: &'a [RlAesKey],
) -> Option<&'a RlAesKey> {
    const CHECK_SIZE: usize = 64;
    let first_few_blocks: [u8; CHECK_SIZE] =
        encrypted_tables_data[..CHECK_SIZE].try_into().unwrap();

    for key in keys {
        let mut check = first_few_blocks.clone();
        key.decrypt(&mut check);
        let mut cursor = Cursor::new(&check);
        if FString::deserialize(&mut cursor, summary.v33()).is_ok() {
            return Some(key);
        }
    }

    None
}

#[derive(Debug, Clone)]
struct NameSwap {
    from: String,
    to: String,
}

impl NameSwap {
    /// returns the padded version if paddable, otherwise None
    fn padded(&self) -> Option<String> {
        let amount_to_pad = self.from.len().checked_sub(self.to.len());
        let Some(amount_to_pad) = amount_to_pad else {
            return None;
        };

        let padding = "\0".repeat(amount_to_pad);
        Some(format!("{}{padding}", self.to))
    }
}

#[derive(Debug, Clone)]
struct FHeaderEncryptedRegion {
    names: Vec<FNameEntry>,
    imports: Vec<FImportEntry>,
    exports: Vec<FExportEntry>,
    compressed_chunk_info: TArray<FCompressedChunkInfo>,
    swaps: Vec<NameSwap>,
}

impl FHeaderEncryptedRegion {
    fn decrypt<'a>(
        summary: &FPackageFileSummary,
        global_reader: &mut (impl Read + Seek),
        keys: &'a [RlAesKey],
    ) -> Result<(Self, &'a RlAesKey)> {
        global_reader
            .seek(SeekFrom::Start(summary.name_offset as u64))
            .unwrap();

        let mut tables_data = vec![0u8; summary.encrypted_region_size() as usize];
        global_reader
            .read_exact(&mut tables_data)
            .context("reading encrypted region data")?;

        let key =
            find_valid_aes_key(summary, &tables_data, keys).context("finding valid aes key")?;

        key.decrypt(&mut tables_data);
        let mut tables_reader = Cursor::new(tables_data);
        let v33 = summary.v33();

        let mut names = Vec::with_capacity(summary.name_count as usize);
        for _ in 0..summary.name_count {
            let entry =
                FNameEntry::deserialize(&mut tables_reader, v33).context("reading name entries")?;
            names.push(entry);
        }

        let mut imports = Vec::with_capacity(summary.import_count as usize);
        for _ in 0..summary.import_count {
            let entry = FImportEntry::deserialize(&mut tables_reader, v33)
                .context("reading import entries")?;
            imports.push(entry);
        }

        let mut exports = Vec::with_capacity(summary.export_count as usize);
        for _ in 0..summary.export_count {
            let entry = FExportEntry::deserialize(&mut tables_reader, v33)
                .context("reading export entries")?;
            exports.push(entry);
        }

        let compressed_chunk_info = TArray::deserialize(&mut tables_reader, v33)
            .context("reading compressed chunk info")?;

        Ok((
            Self {
                names,
                imports,
                exports,
                compressed_chunk_info,
                swaps: Vec::new(),
            },
            key,
        ))
    }

    fn encrypt(
        &self,
        summary_size: i32,
        summary_padding_size: i32,
        new_summary: &mut FPackageFileSummary,
        key: &RlAesKey,
    ) -> Result<Vec<u8>> {
        let v33 = new_summary.v33();
        let current_global_offset = |header: &mut Cursor<Vec<u8>>| {
            header.stream_position().unwrap() as i32 + summary_size + summary_padding_size
        };

        let mut header = Cursor::new(Vec::new());

        new_summary.name_offset = current_global_offset(&mut header);
        let mut new_names = self.names.clone();
        for name in &mut new_names {
            for swap in &self.swaps {
                if name.name.inner == swap.from {
                    name.name.inner = swap.padded().unwrap_or_else(|| swap.to.clone());
                }
            }

            name.serialize(&mut header, v33).unwrap();
        }

        new_summary.import_offset = current_global_offset(&mut header);
        for import in &self.imports {
            import.serialize(&mut header, v33).unwrap();
        }

        new_summary.export_offset = current_global_offset(&mut header);
        for export in &self.exports {
            export.serialize(&mut header, v33).unwrap();
        }

        new_summary.depends_offset = current_global_offset(&mut header);
        new_summary.compressed_chunk_info_offset = header.position() as i32;
        self.compressed_chunk_info
            .serialize(&mut header, v33)
            .context("FHeaderEncryptedRegion: serializing compressed chunk info")?;

        // aes padding
        let new_header_size = header.position();
        let new_header_size_full = (new_header_size + 15) & !15;
        for i in 0..new_header_size_full - new_header_size {
            let pos = new_header_size + i;
            let byte = (pos % 0xFF) as u8;
            header.write_u8(byte).unwrap();
        }

        let header_size_change = new_header_size_full as i32 - new_summary.encrypted_region_size();
        eprintln!("header size changed by {header_size_change}");
        new_summary.total_header_size =
            new_header_size as i32 + new_summary.name_offset + new_summary.garbage_size;

        let global_to_local_offset = |offset: i32| offset - summary_padding_size - summary_size;

        let mut new_exports = self.exports.clone();
        for export in &mut new_exports {
            export.serial_offset += header_size_change as i64;
        }
        header.set_position(global_to_local_offset(new_summary.export_offset) as u64);
        for export in new_exports {
            export.serialize(&mut header, v33).unwrap();
        }

        header.set_position(new_summary.compressed_chunk_info_offset as u64);
        let mut new_compressed_chunk_info = self.compressed_chunk_info.clone();
        for chunk in &mut new_compressed_chunk_info.inner {
            chunk.compressed_offset += header_size_change as i64;

            // idk why but if you do the one with 0 size it freezes the game
            if chunk.uncompressed_size != 0 {
                chunk.uncompressed_offset += header_size_change as i64;
            }
        }
        new_compressed_chunk_info
            .serialize(&mut header, v33)
            .unwrap();

        let mut encrypted = header.into_inner();
        key.encrypt(&mut encrypted);
        Ok(encrypted)
    }

    fn add_swap(&mut self, swap: NameSwap) {
        self.swaps.push(swap);
    }
}

pub struct Upk<'a> {
    summary: FPackageFileSummary,
    header: FHeaderEncryptedRegion,
    payload: Vec<u8>, // the compressed info
    key: &'a RlAesKey,
    id: &'a ItemId,
}

impl<'a> Upk<'a> {
    pub fn new(
        reader: &mut (impl Read + Seek),
        id: &'a ItemId,
        keys: &'a [RlAesKey],
    ) -> Result<Self> {
        let summary = FPackageFileSummary::deserialize(reader, false)?;
        ensure!(summary.is_valid(), "package file tag isnt valid");

        let (header, key) = FHeaderEncryptedRegion::decrypt(&summary, reader, keys)
            .context("extracting encrypted region")?;

        reader.seek(SeekFrom::Start(
            // header end
            (summary.name_offset + summary.encrypted_region_size()) as u64,
        ))?;
        let mut payload = Vec::new();
        reader.read_to_end(&mut payload)?;

        Ok(Self {
            summary,
            header,
            payload,
            key,
            id,
        })
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        let summary_size = {
            let mut serialized = Vec::new();
            self.summary
                .serialize(&mut serialized, false)
                .context("summary serialize first pass")?;
            serialized.len()
        };
        let mut modified_summary = self.summary.clone(); // will perform surgery after
        let summary_padding_size = self.summary.name_offset - summary_size as i32;
        let encrypted_header = self
            .header
            .encrypt(
                summary_size as i32,
                summary_padding_size,
                &mut modified_summary,
                self.key,
            )
            .context("reserializing encrypted header")?;

        let mut serialized = Vec::new();
        modified_summary
            .serialize(&mut serialized, false)
            .context("Upk: serializing modified summary")?;
        serialized.extend(vec![0u8; summary_padding_size as usize]);
        serialized.extend(&encrypted_header);
        serialized.extend(&self.payload);

        Ok(serialized)
    }

    pub fn open<P: AsRef<Path>>(path: P, id: &'a ItemId, keys: &'a [RlAesKey]) -> Result<Self> {
        let mut file = fs::File::open(path)?;
        Self::new(&mut file, id, keys)
    }

    pub fn pretend_to_be(&mut self, other: &'a Upk) {
        self.header.add_swap(NameSwap {
            from: self.id.id().to_owned(),
            to: other.id.id().to_owned(),
        });
        self.header.add_swap(NameSwap {
            from: self.id.sf_name(),
            to: other.id.sf_name(),
        });
        self.summary.guid = other.summary.guid.clone();
        self.key = other.key;
        self.id = other.id;
    }
}
