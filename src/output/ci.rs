use std::process::ExitCode;

use ascii_table::{Align, AsciiTable};

use crate::{
    http::{Request, Response},
    output::Output,
    script_engine::report::TestsReport,
};

#[derive(Debug, Default)]
pub struct CiOutput {
    error: bool,
}

impl Output for CiOutput {
    fn response(&mut self, _response: &Response, _tests: &TestsReport) -> crate::Result<()> {
        Ok(())
    }

    fn request(&mut self, _request: &Request, _request_name: &str) -> crate::Result<()> {
        Ok(())
    }

    fn tests(&mut self, tests: Vec<(String, String, TestsReport)>) -> crate::Result<()> {
        let mut ascii_table = AsciiTable::default();
        ascii_table.set_max_width(256);
        ascii_table
            .column(0)
            .set_header("File")
            .set_align(Align::Left);
        ascii_table
            .column(1)
            .set_header("Request")
            .set_align(Align::Left);
        ascii_table
            .column(2)
            .set_header("Test")
            .set_align(Align::Left);
        ascii_table
            .column(3)
            .set_max_width(6)
            .set_header("Result")
            .set_align(Align::Center);

        let mut data = vec![];
        let mut total_requests = 0;
        let mut failed_requests = 0;
        for (file, request, tests) in &tests {
            total_requests += 1;
            if tests.is_empty() {
                data.push([file.as_str(), request.as_str(), "NO TESTS FOUND", ""]);
            }
            let mut request_failed = false;
            for (test, result) in tests.all() {
                self.error = self.error || result.is_error();
                request_failed = request_failed || result.is_error();
                let result = if result.is_error() {
                    "FAILED"
                } else {
                    "PASSED"
                };
                data.push([file.as_str(), request.as_str(), test, result]);
            }
            if request_failed {
                failed_requests += 1;
            }
        }

        ascii_table.print(data);
        println!("{total_requests} requests completed, {failed_requests} have failed tests");

        Ok(())
    }

    fn exit_code(&mut self) -> ExitCode {
        if self.error {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        }
    }
}
