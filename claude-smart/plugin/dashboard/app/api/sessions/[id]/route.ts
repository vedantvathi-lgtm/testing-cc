import { NextResponse } from "next/server";
import { deleteSession, readSession } from "@/lib/session-reader";

export const dynamic = "force-dynamic";

export async function GET(
  _req: Request,
  context: { params: Promise<{ id: string }> },
) {
  const { id } = await context.params;
  const detail = await readSession(id);
  if (!detail) {
    return NextResponse.json({ error: "not found" }, { status: 404 });
  }
  return NextResponse.json(detail);
}

export async function DELETE(
  _req: Request,
  context: { params: Promise<{ id: string }> },
) {
  const { id } = await context.params;
  const ok = await deleteSession(id);
  if (!ok) {
    return NextResponse.json({ error: "not found" }, { status: 404 });
  }
  return NextResponse.json({ deleted: id });
}
