// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';

import type { AppInfo } from '#lib/ipc/bindings/AppInfo.ts';

import { formatLegalNotice } from './legal-notice.ts';

const SAMPLE_INFO: AppInfo = {
  name: 'sample',
  version: '1.2.3',
  copyrightHolder: 'Sample Holder',
  license: 'GPL-3.0-or-later',
  repository: 'https://example.org/sample',
};

describe('formatLegalNotice', () => {
  it('names the copyright holder', () => {
    expect(formatLegalNotice(SAMPLE_INFO)).toContain(SAMPLE_INFO.copyrightHolder);
  });

  it('names the licence', () => {
    expect(formatLegalNotice(SAMPLE_INFO)).toContain(SAMPLE_INFO.license);
  });
});
