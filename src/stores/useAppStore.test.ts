import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from './useAppStore';
import { invokeCommand } from '../hooks/useTauriInvoke';
import type { Ticket } from '../types/ticket';

vi.mock('../hooks/useTauriInvoke', () => ({
  invokeCommand: vi.fn(),
}));

const mockedInvoke = vi.mocked(invokeCommand);

const sampleTicket: Ticket = {
  id: 1,
  jira_key: 'TEST-1',
  summary: 'Ticket summary',
  status: 'Open',
  priority: 'High',
  issue_type: 'Task',
  assignee: 'Alice',
  reporter: 'Bob',
  created_at: '2025-01-01T09:00:00Z',
  updated_at: '2025-01-01T09:30:00Z',
  resolved_at: null,
  labels: '',
  project_key: 'TEST',
  category: null,
};

describe('useAppStore', () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
    useAppStore.setState(useAppStore.getInitialState(), true);
  });

  it('fetchTickets stores successful results', async () => {
    mockedInvoke.mockResolvedValueOnce([sampleTicket]);

    await useAppStore.getState().fetchTickets();

    expect(mockedInvoke).toHaveBeenCalledWith('get_all_tickets');
    expect(useAppStore.getState().tickets).toEqual([sampleTicket]);
    expect(useAppStore.getState().error).toBeNull();
  });

  it('fetchTickets stores readable errors', async () => {
    mockedInvoke.mockRejectedValueOnce(new Error('backend unavailable'));

    await useAppStore.getState().fetchTickets();

    expect(useAppStore.getState().tickets).toEqual([]);
    expect(useAppStore.getState().error).toContain('backend unavailable');
  });

  it('triggerSync fails fast when settings are missing', async () => {
    mockedInvoke.mockResolvedValueOnce(null);

    await useAppStore.getState().triggerSync();

    expect(mockedInvoke).toHaveBeenCalledTimes(1);
    expect(mockedInvoke).toHaveBeenCalledWith('load_jira_settings');
    expect(useAppStore.getState().syncStatus).toBe('error');
    expect(useAppStore.getState().syncError).toContain('No Jira settings found');
  });
});
