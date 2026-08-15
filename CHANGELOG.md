# Changelog

## v0.1.1

### Añadido
- La biblioteca se navega **por carpetas**: eliges una carpeta raíz y entras en
  ella nivel a nivel, en vez de ver todos los cómics aplanados en una sola
  rejilla. Botón de *Atrás* para volver.
- Cada carpeta muestra portada propia. Si dentro hay un `cover.jpg` o un
  `folder.png` se usa ese; si no, la portada del primer cómic que contenga.
- **La carpeta elegida se recuerda entre sesiones**: al abrir la app se carga
  sola la última biblioteca usada.

### Cambiado
- Los iconos de la interfaz y los controles de ventana pasan a ser SVG. Antes
  eran caracteres Unicode y su aspecto dependía de las fuentes del sistema: en
  Windows salían con grosores distintos, descentrados o como recuadros vacíos.
- El botón de maximizar muestra el icono de restaurar cuando la ventana ya está
  maximizada, como las aplicaciones nativas.
- Las carpetas que no contienen ningún cómic ya no aparecen en la biblioteca.

### Rendimiento
- Al abrir la biblioteca solo se generan las miniaturas del nivel que estás
  viendo. Antes se decodificaba una portada por cada cómic del árbol completo.

### Seguridad
- La navegación queda confinada a la carpeta raíz elegida: las rutas se
  canonicalizan y se valida que estén dentro de ella.
- No se siguen enlaces simbólicos al recorrer la biblioteca.

### Corregido
- `pnpm-workspace.yaml` no declaraba el campo `packages`, lo que rompía la
  compilación en CI.

## v0.1.0

Primera versión: lector de CBZ/CBR con biblioteca, navegación por teclado y
actualizaciones automáticas.
