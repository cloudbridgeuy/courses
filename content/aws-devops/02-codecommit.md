+++
title = "El origen del código — CodeCommit"
+++

::: warning
Desde el **25 de julio de 2024**, AWS CodeCommit no acepta nuevos clientes. Solo
las cuentas que ya usaban el servicio antes de esa fecha conservan acceso completo.
La cuenta del taller fue creada antes del corte, por lo que todos los laboratorios
funcionan con normalidad — pero **no intente replicar esta sesión en una cuenta
personal nueva**: la opción de crear repositorios no estará disponible. Puede leer
el anuncio oficial en el blog de AWS: [How to migrate your AWS CodeCommit
repository to another Git provider](https://aws.amazon.com/blogs/devops/how-to-migrate-your-aws-codecommit-repository-to-another-git-provider/).
:::

## Pre-requisitos

Complete estos pasos **antes de la sesión**.

### 1. Instalar git

- **Windows**: descargue e instale [Git for Windows](https://git-scm.com/download/win).
- **Mac**: ejecute `xcode-select --install` en la Terminal, o si tiene Homebrew:
  `brew install git` ([git-scm.com/download/mac](https://git-scm.com/download/mac)).

Verifique la instalación:

```bash
git --version
```

### 2. Configuración mínima

```bash
git config --global user.name "Su Nombre"
git config --global user.email su-correo@ejemplo.com
```

### 3. Acceso HTTPS

En la [consola de IAM](https://console.aws.amazon.com/iam/home) → su usuario → pestaña **Security credentials** → sección
**HTTPS Git credentials for AWS CodeCommit** → pulse **Generate credentials**.
Guarde el usuario y la contraseña generados; los necesitará al hacer `git push`.

### 4. Acceso SSH

Genere un par de claves si aún no tiene uno:

```bash
ssh-keygen -t rsa -b 4096
```

Luego, en la [consola de IAM](https://console.aws.amazon.com/iam/home) → su usuario → **Security credentials** →
**SSH keys for AWS CodeCommit** → **Upload SSH public key**. Copie el contenido
de `~/.ssh/id_rsa.pub` y péguelo. Anote el **SSH key ID** que IAM asigna
(comienza con `APKA…`).

Configure `~/.ssh/config`:

```
Host git-codecommit.*.amazonaws.com
  User APKA................
  IdentityFile ~/.ssh/id_rsa
```

Ambas vías tienen el mismo peso en el taller; elija la que prefiera.

::: extra HTTPS vs SSH: ¿cuál elegir?
**HTTPS** es más simple de configurar (solo usuario y contraseña generados en IAM),
pero pide credenciales en cada operación salvo que use un *credential helper*.
**SSH** requiere generar y registrar una clave, pero después autentica de forma
transparente. Para el taller cualquiera de los dos funciona igual de bien.
:::

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
2. En la barra de búsqueda superior, escriba `CodeCommit` y seleccione [**CodeCommit**](https://console.aws.amazon.com/codesuite/codecommit/home) en
   los resultados.
3. Confirme que la región seleccionada (esquina superior derecha) es la misma que le
   indicó el instructor. El taller usa una única región para todos los recursos.

### Crear el repositorio

1. Pulse **Create repository**.
2. En **Repository name**, escriba `taller-aws-<su-nombre>`. Use su primer nombre en
   minúsculas y sin acentos (por ejemplo: `taller-aws-carlos`).
3. En **Description** (opcional), escriba una descripción breve, por ejemplo:
   `Repositorio del taller AWS DevOps — Semana 1`.
4. Deje las demás opciones con sus valores predeterminados y pulse **Create**.

CodeCommit crea el repositorio vacío en segundos y lo lleva a la vista principal del
repositorio.

### Clonar el código desde GitHub

::: extra Los comandos de git que usaremos
- `git clone <url>` — copia un repositorio remoto completo a su máquina, con todo su historial.
- `git remote -v` — lista los remotos configurados y sus URLs.
- `git remote add <nombre> <url>` — registra un remoto adicional bajo un nombre.
- `git push -u <remoto> <rama>` — sube los commits de una rama al remoto, y con `-u` recuerda la asociación para futuros `git push`.
- `git status` — muestra el estado del directorio de trabajo.
- `git log` — muestra el historial de commits.
:::

Clone el repositorio del taller desde GitHub y entre al directorio:

```bash
git clone https://github.com/cloudbridgeuy/courses
cd courses
```

### Conectar el repositorio de CodeCommit

En la vista del repositorio recién creado en la consola, pulse el botón **Clone URL**
y copie la URL correspondiente al método de acceso que configuró:

- **HTTPS**: `https://git-codecommit.<región>.amazonaws.com/v1/repos/taller-aws-<su-nombre>`
- **SSH**: `ssh://git-codecommit.<región>.amazonaws.com/v1/repos/taller-aws-<su-nombre>`

Agregue CodeCommit como remoto adicional:

```bash
git remote add codecommit <url-copiada>
```

Verifique que ambos remotos estén registrados:

```bash
git remote -v
```

Debe ver `origin` (GitHub) y `codecommit`.

::: extra ¿Qué es un repositorio remoto?
Un *remoto* es una copia del repositorio alojada en otro servidor. `origin` es solo
el nombre convencional del remoto desde el que se clonó. Un mismo repositorio local
puede tener varios remotos: aquí GitHub queda como fuente de lectura (`origin`) y
CodeCommit como destino de trabajo (`codecommit`).
:::

### Subir el código

Suba la rama `main` a CodeCommit:

```bash
git push -u codecommit main
```

Con HTTPS, git pedirá el usuario y la contraseña generados en IAM. Con SSH, la
autenticación es transparente gracias al archivo `~/.ssh/config`.

### Explorar la vista del repositorio

1. Una vez subido el código, navega por las pestañas de la vista del repositorio:
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

1. En la pestaña **Branches**, pulse **Create branch**.
2. En **Branch name**, escriba `desarrollo`.
3. En **Branch from**, seleccione `main` (la rama de la que derivará).
4. Pulse **Create branch**.

La rama `desarrollo` aparece ahora en la lista. Comparte todos los commits de `main`
en este momento —es una copia exacta del estado actual.

### Localizar el ID del commit

1. Pulse sobre el nombre de la rama `desarrollo` para abrirla.
2. En la pestaña **Commits**, verá el historial. El **commit ID** es el identificador
    hexadecimal largo que aparece junto a cada commit (por ejemplo:
    `a1b2c3d4e5f6...`). Copie los primeros 8 caracteres —son suficientes para
    identificar un commit de forma única en este repositorio.

---

{#ejercicio-1}
### Ejercicio 1 — Clone, conecte y suba el código

Clone el repositorio `https://github.com/cloudbridgeuy/courses` desde GitHub, cree
un repositorio de CodeCommit llamado `taller-aws-<su-nombre>`, agréguelo como
remoto, y suba la rama `main`.

::: solucion
1. Clone el código y entre al directorio:

   ```bash
   git clone https://github.com/cloudbridgeuy/courses
   cd courses
   ```

2. En la consola de AWS, busque [**CodeCommit**](https://console.aws.amazon.com/codesuite/codecommit/home), pulse **Create repository**, y
   cree `taller-aws-<su-nombre>` (su primer nombre en minúsculas, sin acentos).
3. Copie la **Clone URL** del nuevo repositorio (HTTPS o SSH, según el acceso que
   configuró en los pre-requisitos) y agréguelo como remoto:

   ```bash
   # Variante HTTPS
   git remote add codecommit https://git-codecommit.<región>.amazonaws.com/v1/repos/taller-aws-<su-nombre>

   # Variante SSH
   git remote add codecommit ssh://git-codecommit.<región>.amazonaws.com/v1/repos/taller-aws-<su-nombre>
   ```

4. Verifique los remotos con `git remote -v` — debe ver `origin` (GitHub) y
   `codecommit`.
5. Suba la rama `main`:

   ```bash
   git push -u codecommit main
   ```

   Con HTTPS, git pedirá el usuario y la contraseña generados en IAM.
6. En la [consola de CodeCommit](https://console.aws.amazon.com/codesuite/codecommit/home), abra su repositorio: los archivos aparecen en la
   pestaña **Code**.
:::

---

{#ejercicio-2}
### Ejercicio 2 — Cree una rama, y encuentre su commit

Desde la [consola de CodeCommit](https://console.aws.amazon.com/codesuite/codecommit/home), cree la rama `desarrollo` a partir de `main`. Luego
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
