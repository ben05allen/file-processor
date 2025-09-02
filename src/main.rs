use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
enum ParserState {
    PreBlock,
    CentralBlock,
    PostBlock,
    Finished,
}

trait BlockHandler {
    fn handle(&self, content: &str) -> Result<(), Box<dyn std::error::Error>>;
}

struct PrintHandler {
    label: String,
}

impl PrintHandler {
    fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
        }
    }
}

impl BlockHandler for PrintHandler {
    fn handle(&self, content: &str) -> Result<(), Box<dyn std::error::Error>> {
        if !content.is_empty() {
            println!("=== Start: {} ===", self.label);
            println!("{}", content);
            println!("===  End: {}  ===", self.label);
        }
        Ok(())
    }
}

struct FileParser {
    state: ParserState,
    block_content: String,
    pre_sentinel: String,
    post_sentinel: String,
}

impl FileParser {
    fn new(pre_sentinel: &str, post_sentinel: &str) -> Self {
        Self {
            state: ParserState::PreBlock,
            block_content: String::new(),
            pre_sentinel: pre_sentinel.to_string(),
            post_sentinel: post_sentinel.to_string(),
        }
    }

    fn process_line(
        &mut self,
        line: &str,
        processor: &FileProcessor,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match self.state {
            ParserState::PreBlock => {
                if line.trim() == self.pre_sentinel {
                    processor.pre_handler.handle(&self.block_content)?;
                    self.block_content.clear();
                    self.state = ParserState::CentralBlock;
                } else if line.trim() == self.post_sentinel {
                    processor.pre_handler.handle(&self.block_content)?;
                    self.block_content.clear();
                    self.state = ParserState::PostBlock;
                } else {
                    if !self.block_content.is_empty() {
                        self.block_content.push('\n');
                    }
                    self.block_content.push_str(line);
                }
            }
            ParserState::CentralBlock => {
                if line.trim() == self.post_sentinel {
                    if let Some(ref handler) = processor.central_handler {
                        handler.handle(&self.block_content)?;
                    }
                    self.block_content.clear();
                    self.state = ParserState::PostBlock;
                } else {
                    if !self.block_content.is_empty() {
                        self.block_content.push('\n');
                    }
                    self.block_content.push_str(line);
                }
            }
            ParserState::PostBlock => {
                if !self.block_content.is_empty() {
                    self.block_content.push('\n');
                }
                self.block_content.push_str(line);
            }
            ParserState::Finished => {
                // shouldn't get here
            }
        }
        Ok(())
    }

    fn finish(&mut self, processor: &FileProcessor) -> Result<(), Box<dyn std::error::Error>> {
        match self.state {
            ParserState::PreBlock => {
                processor.pre_handler.handle(&self.block_content)?;
            }
            ParserState::CentralBlock => {
                if let Some(ref handler) = processor.central_handler {
                    handler.handle(&self.block_content)?;
                }
            }
            ParserState::PostBlock => {
                if let Some(ref handler) = processor.post_handler {
                    handler.handle(&self.block_content)?;
                }
            }
            ParserState::Finished => {
                // already finished
            }
        }
        self.state = ParserState::Finished;
        Ok(())
    }
}

struct FileProcessor {
    pre_handler: Box<dyn BlockHandler>,
    central_handler: Option<Box<dyn BlockHandler>>,
    post_handler: Option<Box<dyn BlockHandler>>,
}

impl FileProcessor {
    fn new() -> Self {
        Self {
            pre_handler: Box::new(PrintHandler::new("PRE-BLOCK")),
            central_handler: Some(Box::new(PrintHandler::new("CENTRAL-BLOCK"))),
            post_handler: Some(Box::new(PrintHandler::new("POST-BLOCK"))),
        }
    }

    fn process_file<P: AsRef<Path>>(
        &self,
        path: P,
        pre_sentinel: &str,
        post_sentinel: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::open(path).expect("failed to open file");
        let reader = BufReader::new(file);

        let mut parser = FileParser::new(pre_sentinel, post_sentinel);

        for line in reader.lines() {
            let line = line.expect("failed to read line");
            parser.process_line(&line, self)?;
        }

        parser.finish(self)?;

        Ok(())
    }
}
fn main() {
    let processor = FileProcessor::new();

    processor
        .process_file("example.txt", "--- PRE ---", "--- POST ---")
        .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_all_blocks() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "pre-block line 1").unwrap();
        writeln!(file, "pre_block line 2").unwrap();
        writeln!(file, "*pre*").unwrap();
        writeln!(file, "central-block line 1").unwrap();
        writeln!(file, "*post*").unwrap();
        writeln!(file, "post-block line 1").unwrap();

        let processor = FileProcessor::new();

        processor
            .process_file(file.path(), "*pre*", "*post*")
            .unwrap();
    }

    #[test]
    fn test_pre_block_only() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "Only pre-block content").unwrap();
        writeln!(file, "More pre_block").unwrap();

        let processor = FileProcessor::new();
        processor
            .process_file(file.path(), "*pre*", "*post*")
            .unwrap();
    }
}
