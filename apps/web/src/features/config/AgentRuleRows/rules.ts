export const RULES = [
  {
    key: 'blenderHeadlessOnly',
    title: 'Blender headless only',
    detail: 'Agents run Blender in the background and never open its window',
  },
  {
    key: 'neverCaptureScreen',
    title: 'Never capture the screen',
    detail: 'Visual checks render to files instead of grabbing a window',
  },
  {
    key: 'askBeforeShipping',
    title: 'Ask before shipping',
    detail: 'Ship waits for you to confirm before it writes to the site repo',
  },
  {
    key: 'avatarRigTools',
    settingsOnly: true,
    title: 'Offer avatar and rig tools',
    detail:
      'Off leaves these 7 tools out for the next agent that connects, for a shorter tool list',
  },
] as const;
