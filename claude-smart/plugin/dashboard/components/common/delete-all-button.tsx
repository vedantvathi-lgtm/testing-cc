"use client";

import { useState } from "react";
import { Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";

interface Props {
  label: string;
  confirmMessage: string;
  disabled?: boolean;
  onConfirm: () => Promise<void>;
}

export function DeleteAllButton({
  label,
  confirmMessage,
  disabled,
  onConfirm,
}: Props) {
  const [busy, setBusy] = useState(false);

  const click = async () => {
    if (busy) return;
    if (!confirm(confirmMessage)) return;
    setBusy(true);
    try {
      await onConfirm();
    } finally {
      setBusy(false);
    }
  };

  return (
    <Button
      variant="outline"
      size="sm"
      onClick={click}
      disabled={busy || disabled}
      className="text-destructive hover:bg-destructive/10 hover:text-destructive border-destructive/30"
    >
      <Trash2 className="h-3.5 w-3.5" />
      {busy ? "Deleting…" : label}
    </Button>
  );
}
