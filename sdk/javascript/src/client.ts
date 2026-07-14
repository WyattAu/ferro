import { Calendar, Event, Contact, CreateEventRequest } from './models';

interface FerroClientOptions {
  host: string;
  port: number;
  username: string;
  password: string;
}

export class FerroClient {
  private baseUrl: string;
  private auth: string;

  constructor(options: FerroClientOptions) {
    this.baseUrl = `http://${options.host}:${options.port}`;
    this.auth = Buffer.from(`${options.username}:${options.password}`).toString('base64');
  }

  get calendars(): CalendarManager {
    return new CalendarManager(this);
  }

  get contacts(): ContactManager {
    return new ContactManager(this);
  }

  async get(path: string): Promise<any> {
    const response = await fetch(`${this.baseUrl}${path}`, {
      headers: {
        'Authorization': `Basic ${this.auth}`,
      },
    });
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }
    return response.json();
  }

  async post(path: string, data: any): Promise<any> {
    const response = await fetch(`${this.baseUrl}${path}`, {
      method: 'POST',
      headers: {
        'Authorization': `Basic ${this.auth}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(data),
    });
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }
    return response.json();
  }

  async delete(path: string): Promise<void> {
    const response = await fetch(`${this.baseUrl}${path}`, {
      method: 'DELETE',
      headers: {
        'Authorization': `Basic ${this.auth}`,
      },
    });
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }
  }
}

class CalendarManager {
  constructor(private client: FerroClient) {}

  async list(): Promise<Calendar[]> {
    const data = await this.client.get('/dav/calendars');
    return data.calendars.map((c: any) => Calendar.fromDict(c));
  }

  async get(calendarId: string): Promise<Calendar> {
    const data = await this.client.get(`/dav/calendars/${calendarId}`);
    return Calendar.fromDict(data);
  }

  async create(name: string): Promise<Calendar> {
    const data = await this.client.post('/dav/calendars', { name });
    return Calendar.fromDict(data);
  }

  async delete(calendarId: string): Promise<void> {
    await this.client.delete(`/dav/calendars/${calendarId}`);
  }

  async createEvent(calendarId: string, event: CreateEventRequest): Promise<Event> {
    const data = await this.client.post(`/dav/calendars/${calendarId}/events`, event);
    return Event.fromDict(data);
  }

  async listEvents(calendarId: string): Promise<Event[]> {
    const data = await this.client.get(`/dav/calendars/${calendarId}/events`);
    return data.events.map((e: any) => Event.fromDict(e));
  }
}

class ContactManager {
  constructor(private client: FerroClient) {}

  async list(): Promise<Contact[]> {
    const data = await this.client.get('/dav/contacts');
    return data.contacts.map((c: any) => Contact.fromDict(c));
  }

  async get(contactId: string): Promise<Contact> {
    const data = await this.client.get(`/dav/contacts/${contactId}`);
    return Contact.fromDict(data);
  }

  async create(name: string): Promise<Contact> {
    const data = await this.client.post('/dav/contacts', { name });
    return Contact.fromDict(data);
  }

  async delete(contactId: string): Promise<void> {
    await this.client.delete(`/dav/contacts/${contactId}`);
  }
}
