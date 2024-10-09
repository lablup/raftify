use std::{fs, io, sync::Once};

static INIT: Once = Once::new();

struct RunAtEnd;

impl Drop for RunAtEnd {
    fn drop(&mut self) {
        // 모든 테스트가 완료된 후 실행될 코드
        fs::remove_file(".ip_counter").unwrap_or_else(|e| {
            eprintln!("Failed to remove '.ip_counter' file: {}", e)
        });

        match fs::remove_dir_all("./logs") {
            Ok(_) => println!("'./logs' 디렉터리를 성공적으로 제거했습니다."),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                println!("'./logs' 디렉터리가 존재하지 않습니다.")
            }
            Err(e) => eprintln!("'./logs' 디렉터리를 제거하는 중 오류 발생: {}", e),
        }
    }
}

static RUN_AT_END: RunAtEnd = RunAtEnd;

pub fn register_cleaner() {
    // RUN_AT_END 변수를 초기화하여 Drop이 프로그램 종료 시 실행되도록 보장
    INIT.call_once(|| {
        let _ = &RUN_AT_END;
    });
}
