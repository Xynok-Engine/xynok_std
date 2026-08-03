use std::path::PathBuf;

pub mod toml;

#[xynok_std_proc_macro::global_static_readonly]
struct FileSystem
{
    root_path: PathBuf,
}
impl Default for FileSystem
{
    fn default() -> Self
    {
        // hiện tại, do 1 số file là shader SPIV dc build khi cargo build trigger. output dc nằm
        // trong built folder. Nên ta về mặt lý thuyết luôn cần tới các file này khi chạy, nên là
        // để đồng bộ, path root luôn lấy theo exe file.
        let path = if cfg!(debug_assertions)
        {
            //std::path::PathBuf::from("")
            std::env::current_exe().unwrap_or_else(|e| panic!("Cannot determine root path: {}", e))
        }
        else
        {
            std::env::current_exe().unwrap_or_else(|e| panic!("Cannot determine root path: {}", e))
        };

        Self { root_path: path }
    }
}
/// Đảm bảo convention khi đọc file luôn xuất phát tìm kiếm từ root path chứa file.exe
pub fn read<P: AsRef<std::path::Path>>(path: P) -> std::io::Result<Vec<u8>>
{
    let exe_dir = FileSystem::instance();
    let exe_dir = exe_dir.root_path.parent().unwrap();
    let full_path = exe_dir.join(path);
    std::fs::read(&full_path)
}
