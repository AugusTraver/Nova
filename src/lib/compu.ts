// Ayudas de la sección "La compu" (v0.7.1). Lo importante (qué carpetas se
// aceptan) lo decide Rust; esto es solo para mostrar.

/** Los nombres en castellano de las carpetas de Windows (igual que `nombre_para_mostrar` en Rust). */
const EN_CASTELLANO: Record<string, string> = {
  documents: "Documentos",
  desktop: "Escritorio",
  downloads: "Descargas",
  pictures: "Imágenes",
  music: "Música",
  videos: "Videos",
};

/** "C:\Users\vos\Downloads" → "Descargas"; "D:\Facu\TP" → "TP". */
export function nombreDeCarpeta(ruta: string): string {
  const partes = ruta.split(/[\\/]/).filter(Boolean);
  const ultima = partes[partes.length - 1] ?? ruta;
  return EN_CASTELLANO[ultima.toLowerCase()] ?? ultima;
}
