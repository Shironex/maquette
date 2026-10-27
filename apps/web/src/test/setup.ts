import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/react';
import { afterEach } from 'vitest';

afterEach(() => cleanup());

// jsdom lays nothing out and has no scrollIntoView; tests that care replace this.
Element.prototype.scrollIntoView ??= () => undefined;
