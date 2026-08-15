/**
 * Diálogo de confirmación para acciones que tocan muchos elementos a la vez.
 *
 * Marcar una colección entera puede afectar a cientos de cómics y no hay
 * deshacer, así que conviene decir el número exacto antes de hacerlo.
 */
export default function ConfirmDialog({
  title,
  message,
  confirmLabel,
  onConfirm,
  onCancel,
}: {
  title: string;
  message: string;
  confirmLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  return (
    <div className="modal-backdrop" onClick={onCancel}>
      <div className="modal modal-narrow" onClick={(e) => e.stopPropagation()}>
        <div className="modal-head">
          <h2>{title}</h2>
        </div>
        <p className="modal-text">{message}</p>
        <div className="modal-actions">
          <button className="ghost-btn" onClick={onCancel} autoFocus>
            Cancelar
          </button>
          <button className="primary-btn" onClick={onConfirm}>
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
