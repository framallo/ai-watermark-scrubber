//! Shared fixture: build a spec-valid EPUB seeded with invisible-Unicode
//! carriers and a stray generator meta tag, plus a real binary image entry.

use std::io::Write;
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

/// A real, valid 1x1 PNG so epubcheck accepts the fixture image.
pub const PNG_BYTES: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0B, 0x49, 0x44, 0x41, 0x54, 0x78, 0xDA, 0x63, 0xFC, 0xCF, 0xC0, 0x50,
    0x0F, 0x00, 0x04, 0x85, 0x01, 0x80, 0x84, 0xA9, 0x8C, 0x21, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
    0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

pub fn build_dirty_epub(path: &Path) {
    let file = std::fs::File::create(path).unwrap();
    let mut zw = ZipWriter::new(file);
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    zw.start_file("mimetype", stored).unwrap();
    zw.write_all(b"application/epub+zip").unwrap();

    zw.start_file("META-INF/container.xml", deflated).unwrap();
    zw.write_all(
        br#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles>
</container>"#,
    )
    .unwrap();

    zw.start_file("OEBPS/content.opf", deflated).unwrap();
    zw.write_all(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>
<package xmlns=\"http://www.idpf.org/2007/opf\" version=\"3.0\" unique-identifier=\"bookid\">
  <metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\">
    <dc:identifier id=\"bookid\">urn:uuid:6f1d1c9e-0d3a-4a3e-9b2e-000000000001</dc:identifier>
    <dc:title>Test\u{00A0}Book</dc:title>
    <dc:language>en</dc:language>
    <meta property=\"dcterms:modified\">2026-01-01T00:00:00Z</meta>
    <meta name=\"generator\" content=\"bookmill\" />
  </metadata>
  <manifest>
    <item id=\"nav\" href=\"nav.xhtml\" media-type=\"application/xhtml+xml\" properties=\"nav\"/>
    <item id=\"ch1\" href=\"chapter-001.xhtml\" media-type=\"application/xhtml+xml\"/>
    <item id=\"img\" href=\"img.png\" media-type=\"image/png\"/>
  </manifest>
  <spine><itemref idref=\"ch1\"/></spine>
</package>"
            .as_bytes(),
    )
    .unwrap();

    zw.start_file("OEBPS/nav.xhtml", deflated).unwrap();
    zw.write_all(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>
<html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\">
<head><title>Table of Contents</title><meta charset=\"utf-8\"/><meta name=\"generator\" content=\"bookmill\"/></head>
<body><nav epub:type=\"toc\"><ol><li><a href=\"chapter-001.xhtml\">Chapter\u{00A0}One</a></li></ol></nav></body>
</html>"
            .as_bytes(),
    )
    .unwrap();

    zw.start_file("OEBPS/chapter-001.xhtml", deflated).unwrap();
    zw.write_all(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>
<html xmlns=\"http://www.w3.org/1999/xhtml\"><head><title>Ch 1</title></head>
<body><h1>Chapter One</h1>
<p>This para\u{200B}graph hides a zero\u{200B}width space and a\u{00A0}no-break space\u{FEFF}.</p>
<p>An emoji family \u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467} must survive.</p>
</body></html>"
            .as_bytes(),
    )
    .unwrap();

    zw.start_file("OEBPS/img.png", deflated).unwrap();
    zw.write_all(PNG_BYTES).unwrap();

    zw.finish().unwrap();
}
