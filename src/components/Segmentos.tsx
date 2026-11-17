interface SegmentosProps {
  hechas: number;
  total: number;
  /** Cantidad de segmentos de la barra. */
  cantidad?: number;
}

/** Barra de progreso segmentada, tipo medidor de instrumento: ▮▮▮▮▯▯▯▯▯▯ */
export function Segmentos({ hechas, total, cantidad = 12 }: SegmentosProps) {
  const llenos = total === 0 ? 0 : Math.round((hechas / total) * cantidad);
  return (
    <div className="flex gap-[3px]" aria-label={`${hechas} de ${total}`}>
      {Array.from({ length: cantidad }, (_, i) => (
        <span key={i} className={`h-2.5 w-1.5 ${i < llenos ? "bg-texto" : "bg-linea-fuerte"}`} />
      ))}
    </div>
  );
}
