export interface Calendar {
  id: string;
  name: string;
  description?: string;
}

export namespace Calendar {
  export function fromDict(data: any): Calendar {
    return {
      id: data.id,
      name: data.name,
      description: data.description,
    };
  }
}

export interface Event {
  id: string;
  calendarId: string;
  summary: string;
  start: string;
  end: string;
  description?: string;
}

export namespace Event {
  export function fromDict(data: any): Event {
    return {
      id: data.id,
      calendarId: data.calendar_id,
      summary: data.summary,
      start: data.start,
      end: data.end,
      description: data.description,
    };
  }
}

export interface Contact {
  id: string;
  name: string;
  email?: string;
  phone?: string;
}

export namespace Contact {
  export function fromDict(data: any): Contact {
    return {
      id: data.id,
      name: data.name,
      email: data.email,
      phone: data.phone,
    };
  }
}

export interface CreateEventRequest {
  summary: string;
  start: string;
  end: string;
  description?: string;
}
