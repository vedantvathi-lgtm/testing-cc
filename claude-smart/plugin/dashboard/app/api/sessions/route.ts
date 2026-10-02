import { NextResponse } from "next/server";
import { deleteAllSessions, listSessions } from "@/lib/session-reader";

export const dynamic = "force-dynamic";

export async function GET() {
  const sessions = await listSessions();
  return NextResponse.json({ sessions });
}

export async function DELETE() {
  const deleted = await deleteAllSessions();
  return NextResponse.json({ deleted });
}
