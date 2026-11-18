# Verify Office preview with a complete document

2026-11-18 03:35 UTC

Desktop CI44221254957 passes Linux/Windows and all Mac scenarios except
Office Quick Look. Parser and icon hangs are resolved. The fixture was a
handwritten three-entry DOCX ZIP. Replace it with real macOS textutil output
while retaining the real Quick Look thumbnail assertion and the assertion
that no external app opens. Verify locally, then full hosted CI; the hosted
failure is not confirmed resolved until that run passes.
