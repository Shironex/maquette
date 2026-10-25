import { PLATFORM } from '@/ipc/runtime';

/** What the OS calls the store the keys live in. */
export const KEYCHAIN = PLATFORM === 'windows' ? 'Credential Manager' : 'Keychain';
