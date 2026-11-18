import path from 'node:path';
import { execFileSync } from 'node:child_process';

/** An editor-produced Office package for the native preview spec. */
export function sampleOffice(folder: string): string {
  const document = path.join(folder, 'Launch brief.docx');
  // Include the metadata and relationships a real editor writes, rather
  // than relying on Quick Look tolerating a minimal three-entry ZIP.
  execFileSync('/usr/bin/textutil', ['-convert', 'docx', '-format', 'txt', '-stdin', '-output', document], {
    input: 'Launch brief\nRead this document beside the conversation.\n',
  });
  return document;
}
