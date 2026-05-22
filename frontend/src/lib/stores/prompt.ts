import { writable } from 'svelte/store';

export interface PromptOptions {
  title: string;
  placeholder: string;
  defaultValue?: string;
  validation?: (value: string) => string | null;
  onSubmit: (value: string) => void | Promise<void>;
}

export interface PromptState {
  show: boolean;
  title: string;
  placeholder: string;
  value: string;
  validationError: string | null;
  onSubmit: ((value: string) => void | Promise<void>) | null;
  validation: ((value: string) => string | null) | null;
}

const initialState: PromptState = {
  show: false,
  title: '',
  placeholder: '',
  value: '',
  validationError: null,
  onSubmit: null,
  validation: null
};

export const promptStore = writable<PromptState>(initialState);

export function openPrompt(options: PromptOptions) {
  promptStore.set({
    show: true,
    title: options.title,
    placeholder: options.placeholder,
    value: options.defaultValue || '',
    validationError: null,
    onSubmit: options.onSubmit,
    validation: options.validation || null
  });
}

export function closePrompt() {
  promptStore.set(initialState);
}
