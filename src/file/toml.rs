#[derive(thiserror::Error, Debug)]
pub enum Error
{
    #[error("IO: {0}")]
    Io(#[from] std::io::Error),
    #[error("UTF-8: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("TOML deserialize: {0}")]
    Deserialize(#[from] toml::de::Error),
    #[error("TOML serialize: {0}")]
    Serialize(#[from] toml::ser::Error),

    #[error("TOML read_all_at_folder Failed, Folder contains Zero .toml files")]
    FolderHasZeroTomlFile,
}

/// fn này đọc 1 file .toml và trả về dữ liệu tương ứng
/// struct impl: #[derive(Debug, serde::Serialize, serde::Deserialize)]
pub fn read<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> Result<T, Error>
{
    let bytes = super::read(path)?;
    let text = String::from_utf8(bytes)?;
    let val = toml::from_str(&text)?;
    Ok(val)
}
/// Đọc thư mục và parse tất cả file .toml bên trong, trả về FolderHasZeroTomlFile nếu không có file nào.
pub fn read_all_at<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> Result<Vec<T>, Error>
{
    let exe_dir = super::FileSystem::instance();
    let exe_dir = exe_dir.root_path.parent().unwrap();
    let full_path = exe_dir.join(path);

    let mut out = Vec::new();
    for entry in std::fs::read_dir(&full_path)?
    {
        let entry = entry?;
        let entry_path = entry.path();
        if entry_path.extension().and_then(|s| s.to_str()) != Some("toml")
        {
            continue;
        }
        let bytes = std::fs::read(&entry_path)?;
        let text = String::from_utf8(bytes)?;
        let val = toml::from_str(&text)?;
        out.push(val);
    }

    if out.is_empty()
    {
        return Err(Error::FolderHasZeroTomlFile);
    }
    Ok(out)
}

pub fn write<T: serde::Serialize>(path: &std::path::Path, val: &T) -> Result<(), Error>
{
    let exe_dir = super::FileSystem::instance();
    let exe_dir = exe_dir.root_path.parent().unwrap();
    let full_path = exe_dir.join(path);
    if let Some(parent) = full_path.parent()
    {
        std::fs::create_dir_all(parent)?;
    }
    let text = toml::to_string(val)?;
    std::fs::write(&full_path, text)?;
    Ok(())
}

/// Ghi `val` thẳng tới `full_path` (absolute). Không prefix exe_dir như `write`.
/// Helper public để macro `write_mirrored!` có thể gọi từ crate khác.
pub fn write_abs<T: serde::Serialize>(full_path: &std::path::Path, val: &T) -> Result<(), Error>
{
    if let Some(parent) = full_path.parent()
    {
        std::fs::create_dir_all(parent)?;
    }
    let text = toml::to_string(val)?;
    std::fs::write(full_path, text)?;
    Ok(())
}

/// Wrapper của `write`. Trong debug build, ghi thêm một bản sao vào source tree
/// của binary crate đang gọi (resolve qua `CARGO_MANIFEST_DIR` tại call site).
/// Lỗi của bản mirror bị nuốt — chỉ Result của lần ghi chính (`write`) được trả về.
#[macro_export]
macro_rules! __xynok_std_toml_write_mirrored {
    ($path:expr, $val:expr) => {{
        let __path: &std::path::Path = $path;
        let __val = $val;
        let __res = $crate::file::toml::write(__path, __val);
        #[cfg(debug_assertions)]
        {
            let __src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(__path);
            let _ = $crate::file::toml::write_abs(&__src, __val);
        }
        __res
    }};
}
pub use crate::__xynok_std_toml_write_mirrored as write_mirrored;
