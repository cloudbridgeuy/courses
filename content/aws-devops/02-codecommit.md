+++
title = "El origen del código — CodeCommit"
+++

## El problema del código sin versionar

Imagine que trabaja en equipo sobre los mismos archivos: ¿cómo sabe quién cambió qué
y cuándo? ¿Cómo vuelve al estado de ayer si algo se rompió hoy? ¿Cómo trabaja en una
nueva funcionalidad sin afectar el código que ya funciona? Estos son los problemas que
el control de versiones resuelve.

Un sistema de control de versiones registra cada cambio en el código como un **commit**:
un punto en el tiempo con un autor, una fecha y un mensaje que describe qué se modificó.
El historial completo de commits forma el repositorio. Con él se puede navegar hacia
cualquier punto del pasado, comparar estados, y trabajar en paralelo sobre distintas
líneas de desarrollo llamadas **ramas** (*branches*).

## CodeCommit: repositorios Git administrados en AWS

**AWS CodeCommit** es un servicio de control de versiones compatible con Git, alojado
completamente en AWS. No requiere instalar ni operar ningún servidor: usted crea el
repositorio desde la consola, y AWS se encarga de la disponibilidad, la seguridad, y
los respaldos.

En este taller cada participante trabaja sobre su **propio repositorio individual**. Eso
evita conflictos entre participantes y permite que cada uno avance a su ritmo. El nombre
del repositorio sigue la convención `taller-aws-<su-nombre>`, donde `<su-nombre>` es
su primer nombre en minúsculas y sin acentos (por ejemplo: `taller-aws-maria`).

## Práctica guiada: crear el repositorio y subir el código

### Abrir la consola de CodeCommit

1. Inicie sesión en la consola de AWS en [console.aws.amazon.com](https://console.aws.amazon.com).
2. En la barra de búsqueda superior, escriba `CodeCommit` y seleccione el servicio en
   los resultados.
3. Confirme que la región seleccionada (esquina superior derecha) es la misma que le
   indicó el instructor. El taller usa una única región para todos los recursos.

### Crear el repositorio

4. Pulse **Create repository**.
5. En **Repository name**, escriba `taller-aws-<su-nombre>`. Use su primer nombre en
   minúsculas y sin acentos (por ejemplo: `taller-aws-carlos`).
6. En **Description** (opcional), escriba una descripción breve, por ejemplo:
   `Repositorio del taller AWS DevOps — Semana 1`.
7. Deje las demás opciones con sus valores predeterminados y pulse **Create**.

CodeCommit crea el repositorio vacío en segundos y lo lleva a la vista principal del
repositorio.

### Subir el código de la aplicación

El instructor le proveyó un archivo `.zip` con el código de la aplicación. Siga estos
pasos para cargarlo directamente desde la consola, sin necesidad de instalar Git
localmente.

8. En la vista del repositorio vacío, busque el botón **Add file** y despliegue el
   menú. Seleccione **Upload file**.
9. Pulse **Choose file** y seleccione el primer archivo del `.zip` descomprimido.
   Escriba su nombre y correo en los campos **Author name** y **Email address** —
   aparecerán en el historial de commits.
10. En **Commit message**, escriba `Carga inicial del código de la aplicación`.
11. Pulse **Commit changes**.

> **Nota:** la consola de CodeCommit permite subir un archivo a la vez por este método.
> El instructor indicará si el `.zip` incluye un script auxiliar para cargar múltiples
> archivos vía HTTPS, o si se usará otra vía para la carga completa. Lo importante
> conceptualmente es que cada archivo que sube genera un commit rastreable.

### Explorar la vista del repositorio

12. Una vez subido el código, navega por las pestañas de la vista del repositorio:
    - **Code**: muestra los archivos y carpetas. Pulse cualquier archivo para ver su
      contenido.
    - **Commits**: muestra el historial. Pulse un commit para ver exactamente qué cambió.
    - **Branches**: muestra las ramas existentes. Por ahora solo existe `main`.

## Branching básico: la rama `desarrollo`

Una rama (*branch*) es una línea paralela de desarrollo. Los cambios en una rama no
afectan a las demás hasta que se fusionan explícitamente. La convención habitual es
mantener `main` siempre con código funcional y trabajar los cambios en ramas
separadas antes de incorporarlos.

### Crear la rama `desarrollo` desde la consola

13. En la pestaña **Branches**, pulse **Create branch**.
14. En **Branch name**, escriba `desarrollo`.
15. En **Branch from**, seleccione `main` (la rama de la que derivará).
16. Pulse **Create branch**.

La rama `desarrollo` aparece ahora en la lista. Comparte todos los commits de `main`
en este momento —es una copia exacta del estado actual.

### Localizar el ID del commit

17. Pulse sobre el nombre de la rama `desarrollo` para abrirla.
18. En la pestaña **Commits**, verá el historial. El **commit ID** es el identificador
    hexadecimal largo que aparece junto a cada commit (por ejemplo:
    `a1b2c3d4e5f6...`). Copie los primeros 8 caracteres —son suficientes para
    identificar un commit de forma única en este repositorio.

---

{#ejercicio-1}
### Ejercicio 1 — Cree su repositorio

Cree un repositorio de CodeCommit llamado `taller-aws-<su-nombre>`, y suba al menos
un archivo del código de la aplicación provisto por el instructor.

::: solucion
1. En la consola de AWS, busque **CodeCommit** en la barra de búsqueda y ábralo.
2. Pulse **Create repository**.
3. En **Repository name**, escriba `taller-aws-<su-nombre>` (su primer nombre en
   minúsculas, sin acentos).
4. Pulse **Create** para confirmar la creación.
5. En la vista del repositorio vacío, pulse **Add file → Upload file**.
6. Pulse **Choose file** y seleccione un archivo del `.zip` descomprimido por el
   instructor.
7. Complete los campos **Author name** y **Email address**.
8. En **Commit message**, escriba `Carga inicial del código de la aplicación`.
9. Pulse **Commit changes**. El archivo aparecerá ahora en la vista **Code** del
   repositorio.
:::

---

{#ejercicio-2}
### Ejercicio 2 — Cree una rama, y encuentre su commit

Desde la consola de CodeCommit, cree la rama `desarrollo` a partir de `main`. Luego
localice el ID del commit más reciente en esa rama.

::: solucion
1. En la vista de su repositorio, pulse la pestaña **Branches**.
2. Pulse **Create branch**.
3. En **Branch name**, escriba `desarrollo`.
4. En **Branch from**, seleccione `main`.
5. Pulse **Create branch**. La rama aparece en la lista.
6. Pulse sobre el nombre `desarrollo` para abrirla.
7. Seleccione la pestaña **Commits**. El commit más reciente aparece al tope de la
   lista.
8. El **commit ID** es el identificador hexadecimal largo junto al commit. Los primeros
   8 caracteres son suficientes para identificarlo de forma única. Anótelos — los
   usará como referencia en la Semana 2.
:::

:::slide light
{{ejercicio-1}}
:::

:::slide light
{{ejercicio-2}}
:::
