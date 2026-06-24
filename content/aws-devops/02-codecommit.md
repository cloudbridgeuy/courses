+++
title = "El origen del código — CodeCommit"
+++

::: warning
Desde el **25 de julio de 2024**, AWS CodeCommit no acepta nuevos clientes. Solo
las cuentas que ya usaban el servicio antes de esa fecha conservan acceso completo.
La cuenta del taller fue creada antes del corte, por lo que todos los laboratorios
funcionan con normalidad — pero **no intentar replicar esta sesión en una cuenta
personal nueva**: la opción de crear repositorios no estará disponible. Se puede leer
el anuncio oficial en el blog de AWS: [How to migrate your AWS CodeCommit
repository to another Git provider](https://aws.amazon.com/blogs/devops/how-to-migrate-your-aws-codecommit-repository-to-another-git-provider/).
:::

## Pre-requisitos

Completar estos pasos **antes de la sesión**.

### 1. Instalar git

- **Windows**: descargar e instalar [Git for Windows](https://git-scm.com/download/win).
- **Mac**: ejecutar `xcode-select --install` en la Terminal, o si se tiene Homebrew:
  `brew install git` ([git-scm.com/download/mac](https://git-scm.com/download/mac)).

Verificar la instalación:

```bash
git --version
```

### 2. Configuración mínima

```bash
git config --global user.name "Su Nombre"
git config --global user.email su-correo@ejemplo.com
```

Hay tres vías de acceso. Elegir la que corresponda a la cuenta.

::: warning
**Las opciones 3 y 4 requieren un usuario IAM.** Si la organización usa AWS Identity
Center (SSO) para iniciar sesión, la identidad es federada y no existe un usuario IAM
— esas dos opciones no estarán disponibles. En ese caso, ir directamente a la
opción 5.
:::

### 3. Acceso HTTPS (cuenta con usuario IAM)

En la [consola de IAM](https://console.aws.amazon.com/iam/home) → el usuario → pestaña **Security credentials** → sección
**HTTPS Git credentials for AWS CodeCommit** → pulsar **Generate credentials**.
Guardar el usuario y la contraseña generados; se necesitarán al hacer `git push`.

### 4. Acceso SSH (cuenta con usuario IAM)

Generar un par de claves si aún no se tiene uno:

```bash
ssh-keygen -t rsa -b 4096
```

Luego, en la [consola de IAM](https://console.aws.amazon.com/iam/home) → el usuario → **Security credentials** →
**SSH keys for AWS CodeCommit** → **Upload SSH public key**. Copiar el contenido
de `~/.ssh/id_rsa.pub` y pegarlo. Anotar el **SSH key ID** que IAM asigna
(comienza con `APKA…`).

Configurar `~/.ssh/config`:

```
Host git-codecommit.*.amazonaws.com
  User APKA................
  IdentityFile ~/.ssh/id_rsa
```

### 5. Acceso con IAM Identity Center (SSO)

Si se inicia sesión con **AWS IAM Identity Center** (antes AWS SSO), la identidad es
federada y **no existe un usuario IAM**, por lo que las opciones 3 y 4 no aplican.
Usar `git-remote-codecommit`, que autentica con las credenciales del perfil del
AWS CLI.

1. Instalar **AWS CLI v2** ([guía oficial](https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html)).
2. Configurar un perfil con SSO y anotar el **nombre del perfil** asignado:

   ```bash
   aws configure sso
   ```

3. Instalar el ayudante de git (requiere Python):

   ```bash
   pip install git-remote-codecommit
   ```

4. Con esta vía, las URLs de CodeCommit toman la forma
   `codecommit::<región>://<perfil>@<nombre-del-repositorio>`; no se necesitan
   credenciales HTTPS ni claves SSH.

Elegir la vía que corresponda a la cuenta. Si se dispone de un usuario IAM, cualquiera de las
opciones 3, 4 o 5 funciona; si se usa Identity Center, usar la opción 5.

::: extra HTTPS, SSH o Identity Center: ¿cuál elegir?
**HTTPS** es la más simple de configurar (solo usuario y contraseña generados en
IAM), pero pide credenciales en cada operación salvo que se use un *credential helper*.
**SSH** requiere generar y registrar una clave, pero después autentica de forma
transparente. Ambas necesitan un **usuario IAM**. Si la cuenta usa **Identity
Center**, no existe usuario IAM: la única vía es `git-remote-codecommit` (opción 5),
que reutiliza la sesión del AWS CLI. Para el taller, usar la que corresponda a cómo
se inicia sesión en AWS.
:::

## El problema del código sin versionar

Imagine que se trabaja en equipo sobre los mismos archivos: ¿cómo saber quién cambió qué
y cuándo? ¿Cómo volver al estado de ayer si algo se rompió hoy? ¿Cómo trabajar en una
nueva funcionalidad sin afectar el código que ya funciona? Estos son los problemas que
el control de versiones resuelve.

Un sistema de control de versiones registra cada cambio en el código como un **commit**:
un punto en el tiempo con un autor, una fecha y un mensaje que describe qué se modificó.
El historial completo de commits forma el repositorio. Con él se puede navegar hacia
cualquier punto del pasado, comparar estados, y trabajar en paralelo sobre distintas
líneas de desarrollo llamadas **ramas** (*branches*).

## CodeCommit: repositorios Git administrados en AWS

**AWS CodeCommit** es un servicio de control de versiones compatible con Git, alojado
completamente en AWS. No requiere instalar ni operar ningún servidor: se crea el
repositorio desde la consola, y AWS se encarga de la disponibilidad, la seguridad, y
los respaldos.

En este taller cada participante trabaja sobre su **propio repositorio individual**. Eso
evita conflictos entre participantes y permite avanzar a ritmo propio. El nombre
del repositorio sigue la convención `taller-aws-<su-nombre>`, donde `<su-nombre>` es
el primer nombre en minúsculas y sin acentos (por ejemplo: `taller-aws-maria`).

## Práctica guiada: crear el repositorio y subir el código

### Abrir la consola de CodeCommit

1. Iniciar sesión en la consola de AWS en [console.aws.amazon.com](https://console.aws.amazon.com).
2. Abrir [**CodeCommit**](https://console.aws.amazon.com/codesuite/codecommit/home).
3. Confirmar que la región seleccionada (esquina superior derecha) es la misma que
   indicó el instructor. El taller usa una única región para todos los recursos.

### Crear el repositorio

1. Pulsar **Create repository**.
2. En **Repository name**, escribir `taller-aws-<su-nombre>`. Usar el primer nombre en
   minúsculas y sin acentos (por ejemplo: `taller-aws-carlos`).
3. En **Description** (opcional), escribir una descripción breve, por ejemplo:
   `Repositorio del taller AWS DevOps — Semana 1`.
4. Dejar las demás opciones con sus valores predeterminados y pulsar **Create**.

CodeCommit crea el repositorio vacío en segundos y lleva a la vista principal del
repositorio.

### Clonar el código desde GitHub

::: extra Los comandos de git que se usarán
- `git clone <url>` — copia un repositorio remoto completo a la máquina, con todo su historial.
- `git remote -v` — lista los remotos configurados y sus URLs.
- `git remote add <nombre> <url>` — registra un remoto adicional bajo un nombre.
- `git push -u <remoto> <rama>` — sube los commits de una rama al remoto, y con `-u` recuerda la asociación para futuros `git push`.
- `git status` — muestra el estado del directorio de trabajo.
- `git log` — muestra el historial de commits.
:::

Clonar el repositorio del taller desde GitHub y entrar al directorio:

```bash
git clone https://github.com/cloudbridgeuy/courses
cd courses
```

### Conectar el repositorio de CodeCommit

En la vista del repositorio recién creado en la consola, pulsar el botón **Clone URL**
y copiar la URL correspondiente al método de acceso configurado:

- **HTTPS**: `https://git-codecommit.<región>.amazonaws.com/v1/repos/taller-aws-<su-nombre>`
- **SSH**: `ssh://git-codecommit.<región>.amazonaws.com/v1/repos/taller-aws-<su-nombre>`
- **Identity Center (grc)**: `codecommit::<región>://<perfil>@taller-aws-<su-nombre>`
  (no aparece en el botón **Clone URL**; construirla con la región y el nombre del perfil).

Agregar CodeCommit como remoto adicional:

```bash
git remote add codecommit <url-copiada>
```

Verificar que ambos remotos estén registrados:

```bash
git remote -v
```

Debe verse `origin` (GitHub) y `codecommit`.

::: extra ¿Qué es un repositorio remoto?
Un *remoto* es una copia del repositorio alojada en otro servidor. `origin` es solo
el nombre convencional del remoto desde el que se clonó. Un mismo repositorio local
puede tener varios remotos: aquí GitHub queda como fuente de lectura (`origin`) y
CodeCommit como destino de trabajo (`codecommit`).
:::

### Subir el código

Subir la rama `main` a CodeCommit:

```bash
git push -u codecommit main
```

Con HTTPS, git pedirá el usuario y la contraseña generados en IAM. Con SSH, la
autenticación es transparente gracias al archivo `~/.ssh/config`.

### Explorar la vista del repositorio

1. Una vez subido el código, navegar por las pestañas de la vista del repositorio:
    - **Code**: muestra los archivos y carpetas. Pulsar cualquier archivo para ver su
      contenido.
    - **Commits**: muestra el historial. Pulsar un commit para ver exactamente qué cambió.
    - **Branches**: muestra las ramas existentes. Por ahora solo existe `main`.

## Branching básico: la rama `dev`

Una rama (*branch*) es una línea paralela de desarrollo. Los cambios en una rama no
afectan a las demás hasta que se fusionan explícitamente. La convención habitual es
mantener `main` siempre con código funcional y trabajar los cambios en ramas
separadas antes de incorporarlos.

### Crear la rama `dev` desde la consola

1. En la pestaña **Branches**, pulsar **Create branch**.
2. En **Branch name**, escribir `dev`.
3. En **Branch from**, seleccionar `main` (la rama de la que derivará).
4. Pulsar **Create branch**.

La rama `dev` aparece ahora en la lista. Comparte todos los commits de `main`
en este momento —es una copia exacta del estado actual.

### Localizar el ID del commit

1. Pulsar sobre el nombre de la rama `dev` para abrirla.
2. En la pestaña **Commits**, se verá el historial. El **commit ID** es el identificador
    hexadecimal largo que aparece junto a cada commit (por ejemplo:
    `a1b2c3d4e5f6...`). Copiar los primeros 8 caracteres —son suficientes para
    identificar un commit de forma única en este repositorio.

---

{#ejercicio-1}
### Ejercicio 1 — Clonar, conectar y subir el código

Clonar el repositorio `https://github.com/cloudbridgeuy/courses` desde GitHub, crear
un repositorio de CodeCommit llamado `taller-aws-<su-nombre>`, agregarlo como
remoto, y subir la rama `main`.

::: solucion
1. Clonar el código y entrar al directorio:

   ```bash
   git clone https://github.com/cloudbridgeuy/courses
   cd courses
   ```

2. Abrir [**CodeCommit**](https://console.aws.amazon.com/codesuite/codecommit/home), pulsar **Create repository**, y
   crear `taller-aws-<su-nombre>` (el primer nombre en minúsculas, sin acentos).
3. Agregar CodeCommit como remoto, según el acceso configurado en los
   pre-requisitos:

   ```bash
   # Variante HTTPS (usuario IAM) — copiar la Clone URL del repositorio
   git remote add codecommit https://git-codecommit.<región>.amazonaws.com/v1/repos/taller-aws-<su-nombre>

   # Variante SSH (usuario IAM) — copiar la Clone URL del repositorio
   git remote add codecommit ssh://git-codecommit.<región>.amazonaws.com/v1/repos/taller-aws-<su-nombre>

   # Variante Identity Center (grc) — construirla con la región y el perfil
   git remote add codecommit codecommit::<región>://<perfil>@taller-aws-<su-nombre>
   ```

4. Verificar los remotos con `git remote -v` — deben verse `origin` (GitHub) y
   `codecommit`.
5. Subir la rama `main`:

   ```bash
   git push -u codecommit main
   ```

   Con HTTPS, git pedirá el usuario y la contraseña generados en IAM. Con
   Identity Center (grc), se reutiliza la sesión activa del AWS CLI.
6. En la [consola de CodeCommit](https://console.aws.amazon.com/codesuite/codecommit/home), abrir el repositorio: los archivos aparecen en la
   pestaña **Code**.
:::

---

{#ejercicio-2}
### Ejercicio 2 — Crear una rama y encontrar su commit

Desde la [consola de CodeCommit](https://console.aws.amazon.com/codesuite/codecommit/home), crear la rama `dev` a partir de `main`. Luego
localizar el ID del commit más reciente en esa rama.

::: solucion
1. En la vista del repositorio, pulsar la pestaña **Branches**.
2. Pulsar **Create branch**.
3. En **Branch name**, escribir `dev`.
4. En **Branch from**, seleccionar `main`.
5. Pulsar **Create branch**. La rama aparece en la lista.
6. Pulsar sobre el nombre `dev` para abrirla.
7. Seleccionar la pestaña **Commits**. El commit más reciente aparece al tope de la
   lista.
8. El **commit ID** es el identificador hexadecimal largo junto al commit. Los primeros
   8 caracteres son suficientes para identificarlo de forma única. Anotarlos — se
   usarán como referencia en la Semana 2.
:::

:::slide light
{{ejercicio-1}}
:::

:::slide light
{{ejercicio-2}}
:::
