use crate::rocket_league::{ItemPackageName, RlAesKey};
use anyhow::{Context as _, Result, anyhow, ensure};
use byteorder::{LittleEndian, ReadBytesExt as _, WriteBytesExt as _};
use std::{
    fs,
    io::{self, Cursor, Read, Seek, SeekFrom, Write},
    path::Path,
};

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
    // nonces n stuff
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

    pub fn extra_encryption(&self) -> bool {
        (self.package_flags & 0x0800) != 0
    }

    pub fn set_extra_encryption(&mut self, on: bool) {
        if on {
            self.package_flags |= 0x0800;
        } else {
            self.package_flags &= !0x0800;
        }
    }
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

const NONCE_SIZE: i32 = 12;

#[derive(Debug, Clone)]
struct FHeaderEncryptedRegion {
    nonce: Option<[u8; NONCE_SIZE as usize]>,
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
        key: &'a RlAesKey,
    ) -> Result<Self> {
        let nonce = summary.v33().then(|| {
            let mut buffer = [0u8; 12];
            global_reader
                .seek(SeekFrom::Start(
                    summary.name_offset as u64 - NONCE_SIZE as u64,
                ))
                .unwrap();
            global_reader.read_exact(&mut buffer).unwrap();
            buffer
        });

        let mut tables_data = vec![0u8; summary.encrypted_region_size() as usize];
        global_reader
            .read_exact(&mut tables_data)
            .context("reading encrypted region data")?;

        if summary.extra_encryption()
            && let nonce = nonce.as_ref().ok_or(anyhow!(
                "extra encryption is enabled but licensee version is <33"
            ))?
        {
            key.ctr(&mut tables_data, &nonce);
        } else {
            key.decrypt(&mut tables_data);
        }

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

        Ok(Self {
            nonce,
            names,
            imports,
            exports,
            compressed_chunk_info,
            swaps: Vec::new(),
        })
    }

    fn encrypt(
        &self,
        summary_size: i32,
        new_summary: &mut FPackageFileSummary,
        key: &RlAesKey,
    ) -> Result<Vec<u8>> {
        let v33 = new_summary.v33();
        let current_global_offset = |header: &mut Cursor<Vec<u8>>| {
            header.stream_position().unwrap() as i32 + summary_size + NONCE_SIZE
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

        let new_header_size = header.position();
        let new_header_size_full = (new_header_size + 15) & !15;
        {
            let aes_padding_size = new_header_size_full - new_header_size;
            if new_summary.extra_encryption() {
                // boring padding
                header.write(&vec![0u8; aes_padding_size as usize]).unwrap();
            } else {
                // cool padding
                for i in 0..new_header_size_full - new_header_size {
                    let pos = new_header_size + i;
                    let byte = (pos % 0xFF) as u8;
                    header.write_u8(byte).unwrap();
                }
            }
        }

        let header_size_change = new_header_size_full as i32 - new_summary.encrypted_region_size();
        new_summary.total_header_size =
            new_header_size as i32 + new_summary.name_offset + new_summary.garbage_size;

        let global_to_local_offset = |offset: i32| offset - NONCE_SIZE - summary_size;

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
        if new_summary.extra_encryption()
            && let nonce = self.nonce.as_ref().ok_or(anyhow!(
                "extra encryption is enabled but licensee version is <33"
            ))?
        {
            key.ctr(&mut encrypted, nonce);
        } else {
            key.encrypt(&mut encrypted);
        }

        let mut serialized = Vec::with_capacity(encrypted.len() + NONCE_SIZE as usize);
        if new_summary.v33()
            && let Some(nonce) = &self.nonce
        {
            serialized.extend_from_slice(nonce);
        }
        serialized.extend_from_slice(&encrypted);
        Ok(serialized)
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
    id: &'a ItemPackageName,
}

impl<'a> Upk<'a> {
    pub fn new(
        reader: &mut (impl Read + Seek),
        id: &'a ItemPackageName,
        key: &'a RlAesKey,
    ) -> Result<Self> {
        let summary = FPackageFileSummary::deserialize(reader, false)?;
        ensure!(summary.is_valid(), "package file tag isnt valid");

        let header = FHeaderEncryptedRegion::decrypt(&summary, reader, key)
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
        let encrypted_header = self
            .header
            .encrypt(summary_size as i32, &mut modified_summary, self.key)
            .context("reserializing encrypted header")?;

        let mut serialized = Vec::new();
        modified_summary
            .serialize(&mut serialized, false)
            .context("Upk: serializing modified summary")?;
        serialized.extend(&encrypted_header);
        serialized.extend(&self.payload);

        Ok(serialized)
    }

    pub fn open<P: AsRef<Path>>(
        path: P,
        id: &'a ItemPackageName,
        key: &'a RlAesKey,
    ) -> Result<Self> {
        let mut file = fs::File::open(path)?;
        Self::new(&mut file, id, key)
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
        self.id = other.id;

        // note: do this BEFORE setting self.key/self.nonce
        if self.summary.extra_encryption() {
            // decrypts
            self.run_ctr_on_payload();
        }

        if self.summary.extra_encryption() && !other.summary.extra_encryption() {
            for chunk in &mut self.header.compressed_chunk_info.inner {
                chunk.nonce = Some([0u8; 12]);
            }
        }

        self.key = other.key;
        if self.header.nonce.is_some()
            && let Some(new_nonce) = &other.header.nonce
        {
            self.header.nonce.replace(new_nonce.clone());
        }

        if other.summary.extra_encryption() {
            // re-encrypts
            self.run_ctr_on_payload();
        }

        self.summary
            .set_extra_encryption(other.summary.extra_encryption());
    }

    fn run_ctr_on_payload(&mut self) {
        let global_to_local_pos = |position: i32| {
            position - (self.summary.name_offset + self.summary.encrypted_region_size())
        };

        for chunk in &self.header.compressed_chunk_info.inner {
            let start = global_to_local_pos(chunk.compressed_offset as i32) as usize;
            let end = start + chunk.compressed_size as usize;
            self.key
                .ctr(&mut self.payload[start..end], chunk.nonce.as_ref().unwrap());
        }
    }
}
