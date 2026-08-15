# Changelog

## v0.1.3

### Añadido
- **Detección de colecciones.** Al entrar en la carpeta de una serie, la app
  lee los metadatos que muchos cómics traen dentro del propio archivo
  (`ComicInfo.xml`) y te dice qué números tienes y cuáles faltan:
  *«Tienes 6 de 12 — Faltan: 4, 6-7, 10-12»*. No hace falta conexión ni cuenta
  en ningún sitio.
- **Conexión opcional con Metron**, una base de datos de cómics comunitaria y
  abierta, para completar el total de números cuando el archivo no lo declara.
  Se conecta con un token desde el botón *Metron*, y solo consulta cuando lo
  pides tú: la aplicación nunca envía nada por su cuenta. El token se guarda en
  el almacén de credenciales del sistema, no en ningún fichero de la app.
- **Selector de bibliotecas**: un desplegable con las que tengas guardadas, para
  cambiar de una a otra sin volver a buscar la carpeta.
- El número de cada cómic aparece en su tarjeta cuando el archivo lo indica.
- Icono propio de la aplicación, en lugar del genérico que traía.

### Rendimiento
- Volver a una carpeta ya visitada es mucho más rápido: antes se abría cada
  archivo de nuevo para contar sus páginas aunque la portada ya estuviera
  guardada. Ahora se guardan juntos portada, páginas y metadatos.
- El recuento de cómics de cada carpeta también se guarda, en vez de recorrer
  todo su contenido cada vez que se mira.

## v0.1.2

### Añadido
- **Zoom en el lector**, del 100% al 400%: con `Ctrl` + rueda, con los botones
  de la barra o con las teclas `+`, `−` y `0` para restablecer.
- **Navegación vertical con las flechas arriba y abajo.** Con la página
  ampliada, las flechas recorren la propia página y solo cambian de página al
  llegar al borde; si la página cabe entera en pantalla, pasan directamente.
  La barra espaciadora hace lo mismo que la flecha abajo.
- **Marcar cómics como leídos**: automáticamente al llegar a la última página,
  y a mano desde el lector o desde el botón que aparece sobre cada portada. Los
  leídos se muestran atenuados y con un distintivo.
- Logotipo propio en la barra de título.

### Cambiado
- Al retroceder de página, la vista aparece por la parte de abajo, que es por
  donde se venía leyendo, en vez de saltar al principio.
- Al cerrar un cómic se vuelve a la carpeta donde estabas, no a la raíz de la
  biblioteca.
- Un cómic que no se puede leer ya no desaparece de la biblioteca: se muestra
  atenuado, con un aviso y el motivo del fallo.

### Rendimiento
- La biblioteca aparece al instante aunque tenga miles de cómics: el listado ya
  no abre ningún archivo, y las portadas se generan solo para lo que estás
  viendo.
- Las miniaturas se guardan en caché en disco, así que volver a una carpeta ya
  visitada es inmediato.

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
