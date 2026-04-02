const version =
  process.env.SAICODE_LOCAL_VERSION ??
  process.env.CLAUDE_CODE_LOCAL_VERSION ??
  '1.0.0';
const packageUrl =
  process.env.SAICODE_LOCAL_PACKAGE_URL ??
  process.env.CLAUDE_CODE_LOCAL_PACKAGE_URL ??
  'saicode';
const buildTime =
  process.env.SAICODE_LOCAL_BUILD_TIME ??
  process.env.CLAUDE_CODE_LOCAL_BUILD_TIME ??
  new Date().toISOString();

process.env.CLAUDE_CODE_LOCAL_SKIP_REMOTE_PREFETCH ??= '1';

Object.assign(globalThis, {
  MACRO: {
    VERSION: version,
    PACKAGE_URL: packageUrl,
    NATIVE_PACKAGE_URL: packageUrl,
    BUILD_TIME: buildTime,
    FEEDBACK_CHANNEL: 'local',
    VERSION_CHANGELOG: '',
    ISSUES_EXPLAINER: '',
  },
});
