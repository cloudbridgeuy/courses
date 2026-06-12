+++
title = "Integración continua — CodeBuild, y ECR"
+++

## El problema: construir de forma reproducible

En el repositorio ya tiene el código fuente. Pero entre el código fuente y una
aplicación en ejecución hay un paso crítico: **la construcción** (*build*). Para una
aplicación empaquetada como contenedor Docker, eso significa ejecutar `docker build`,
etiquetar la imagen, y dejarla en un lugar donde el sistema de despliegue pueda
encontrarla.

Si ese proceso lo ejecuta manualmente en su máquina, depende de lo que tenga instalado,
de la versión del sistema operativo, de si recuerda los mismos comandos que la última
vez. La integración continua resuelve esto ejecutando el build en un entorno
**estándar, limpio, y reproducible** cada vez —sin intervención manual.

## CodeBuild: build administrado

**AWS CodeBuild** es un servicio de build totalmente administrado. Usted le indica la
fuente (su repositorio de CodeCommit), la imagen del entorno de construcción (una imagen
estándar con las herramientas que necesita), y los comandos a ejecutar (descritos en un
archivo `buildspec.yml`). CodeBuild aprovisiona el entorno, ejecuta los comandos, y
libera los recursos al terminar. No hay servidores que mantener.

## Amazon ECR: el registro de imágenes Docker

Una vez construida la imagen Docker, hay que guardarla en algún lugar. **Amazon ECR**
(Elastic Container Registry) es el servicio de AWS para almacenar imágenes Docker de
forma privada y segura. Cuando ECS necesite lanzar su contenedor en la Semana 3, irá a
buscar la imagen directamente a ECR.

El flujo completo de esta sección es:

```
CodeCommit (código fuente)
    ↓
CodeBuild (lee buildspec.yml, construye la imagen)
    ↓
ECR (almacena la imagen con una etiqueta)
```

## El archivo `buildspec.yml`

El archivo `buildspec.yml` está incluido en el `.zip` de la aplicación, en la raíz del
proyecto. Es el contrato entre su código y CodeBuild: describe exactamente qué comandos
ejecutar en cada fase del build.

El archivo tiene este aspecto:

```yaml
version: 0.2

phases:
  pre_build:
    commands:
      - echo Autenticando con Amazon ECR...
      - aws ecr get-login-password --region $AWS_DEFAULT_REGION |
          docker login --username AWS --password-stdin
          $AWS_ACCOUNT_ID.dkr.ecr.$AWS_DEFAULT_REGION.amazonaws.com
      - IMAGE_URI=$AWS_ACCOUNT_ID.dkr.ecr.$AWS_DEFAULT_REGION.amazonaws.com/$IMAGE_REPO_NAME
  build:
    commands:
      - echo Construyendo la imagen Docker...
      - docker build -t $IMAGE_REPO_NAME:$IMAGE_TAG .
      - docker tag $IMAGE_REPO_NAME:$IMAGE_TAG $IMAGE_URI:$IMAGE_TAG
  post_build:
    commands:
      - echo Publicando la imagen en ECR...
      - docker push $IMAGE_URI:$IMAGE_TAG
      - echo Build completado.
```

Las tres fases y su propósito:

| Fase | Propósito |
|------|-----------|
| `pre_build` | Preparación: autenticar con ECR para tener permiso de publicar imágenes. |
| `build` | Construcción: ejecutar `docker build` y etiquetar la imagen con el URI de ECR. |
| `post_build` | Publicación: subir la imagen etiquetada al repositorio de ECR. |

Las variables de entorno (`$AWS_ACCOUNT_ID`, `$AWS_DEFAULT_REGION`, `$IMAGE_REPO_NAME`,
`$IMAGE_TAG`) se definen en el proyecto de CodeBuild, no en el archivo. Esto permite
reutilizar el mismo `buildspec.yml` en distintos entornos sin modificarlo.

## Práctica guiada: crear el repositorio ECR

### Abrir Amazon ECR

1. En la barra de búsqueda de la consola de AWS, escriba `ECR` y seleccione
   [**Elastic Container Registry**](https://console.aws.amazon.com/ecr/home).
2. En el panel lateral, asegúrese de estar en **Private registry → Repositories**.

### Crear el repositorio de imágenes

1. Pulse **Create repository**.
2. En **Repository name**, escriba `taller-aws-<su-nombre>` (el mismo nombre que usó
   en CodeCommit, por consistencia).
3. Deje **Image tag mutability** en **Mutable** — esto permite reutilizar etiquetas
   como `latest` o `v1` entre builds sucesivos.
4. Deje las demás opciones con sus valores predeterminados y pulse **Create repository**.

El repositorio aparece en la lista. Pulse sobre su nombre y copie el **URI** completo
—algo como `123456789012.dkr.ecr.us-east-1.amazonaws.com/taller-aws-maria`. Lo
necesitará al configurar CodeBuild.

## Práctica guiada: crear y ejecutar el proyecto de CodeBuild

### Abrir CodeBuild

1. En la barra de búsqueda, escriba `CodeBuild` y ábralo.
2. Pulse **Create build project**.

### Configurar la fuente

1. En **Project name**, escriba `taller-aws-<su-nombre>-build`.
2. En la sección **Source**, seleccione **Source provider: AWS CodeCommit**.
3. En **Repository**, seleccione su repositorio `taller-aws-<su-nombre>`.
4. En **Reference type**, seleccione **Branch** y elija `main`.

### Configurar el entorno de construcción

1. En la sección **Environment**, seleccione:
    - **Environment image**: **Managed image**
    - **Compute**: **EC2**
    - **Operating system**: **Amazon Linux**
    - **Runtime(s)**: **Standard**
    - **Image**: seleccione la versión más reciente disponible (por ejemplo
      `aws/codebuild/standard:7.0`).
2. Active **Privileged mode** marcando la casilla correspondiente. Este modo es
    **obligatorio** para que CodeBuild pueda ejecutar el daemon de Docker y construir
    imágenes de contenedor.
3. En **Service role**, seleccione **New service role**. CodeBuild creará
    automáticamente un rol de IAM con los permisos básicos. Anote el nombre del rol
    —necesitará agregarle permisos de ECR a continuación.

### Agregar permisos de ECR al rol de CodeBuild

El rol creado automáticamente puede acceder a CodeCommit, pero aún no tiene permiso
para publicar en ECR. Siga estos pasos **antes** de ejecutar el build:

1. En una nueva pestaña del navegador, abra [**IAM → Roles**](https://console.aws.amazon.com/iam/home#/roles) y busque el rol recién
    creado (su nombre comienza con `codebuild-taller-aws-<su-nombre>`).
2. Pulse **Add permissions → Attach policies**.
3. Busque `AmazonEC2ContainerRegistryPowerUser` y selecciónelo.
4. Pulse **Add permissions**. Vuelva a la pestaña de CodeBuild.

### Configurar las variables de entorno

1. En la sección **Environment**, desplácese hasta **Additional configuration →
    Environment variables** y agregue las siguientes variables:

    | Name | Value | Type |
    |------|-------|------|
    | `AWS_ACCOUNT_ID` | Su ID de cuenta AWS (12 dígitos, sin guiones) | Plaintext |
    | `IMAGE_REPO_NAME` | `taller-aws-<su-nombre>` | Plaintext |
    | `IMAGE_TAG` | `latest` | Plaintext |

    > **Tip:** encuentre su ID de cuenta en la esquina superior derecha de la consola,
    > bajo el nombre de su usuario o rol.

### Finalizar la configuración

1. En la sección **Buildspec**, deje seleccionado **Use a buildspec file** —CodeBuild
    buscará automáticamente el archivo `buildspec.yml` en la raíz del repositorio.
2. En la sección **Artifacts**, seleccione **No artifacts** —el resultado del build
    es la imagen publicada en ECR, no un artefacto de archivo.
3. Pulse **Create build project**.

### Ejecutar el build y seguir los logs

1. En la vista del proyecto recién creado, pulse **Start build**.
2. CodeBuild provisiona el entorno y comienza a ejecutar los comandos del
    `buildspec.yml`. La pestaña **Build logs** muestra la salida en tiempo real.
3. Siga los logs. Verá las tres fases: autenticación con ECR, `docker build`, y
    `docker push`. El proceso tarda entre 2 y 5 minutos la primera vez.
4. Al terminar, el estado cambia a **Succeeded** (en verde) o **Failed** (en rojo).
    Si falla, el log indica en qué línea ocurrió el error.

### Verificar la imagen en ECR

1. Vuelva a la [consola de ECR](https://console.aws.amazon.com/ecr/home) y abra su repositorio `taller-aws-<su-nombre>`.
2. En la pestaña **Images**, verá la imagen recién publicada con la etiqueta `latest`
    y la fecha y hora del push. Copie el **Image URI** completo —lo necesitará en la
    siguiente sección para lanzar el stack de CloudFormation.

---

### Ejercicio 3 — Cree el repositorio de imágenes

Cree un repositorio privado en Amazon ECR con el nombre `taller-aws-<su-nombre>`.

::: solucion
1. En la consola de AWS, busque [**ECR**](https://console.aws.amazon.com/ecr/home) y abra **Elastic Container Registry**.
2. En el panel lateral, seleccione **Private registry → Repositories**.
3. Pulse **Create repository**.
4. En **Repository name**, escriba `taller-aws-<su-nombre>`.
5. Deje **Image tag mutability** en **Mutable**.
6. Pulse **Create repository**.
7. En la lista de repositorios, pulse sobre el nombre del repositorio recién creado.
8. Copie el **URI** completo que aparece en la parte superior —lo necesitará para
   configurar CodeBuild y para el parámetro de CloudFormation en la sección siguiente.
:::

---

### Ejercicio 4 — Ejecute su primera build

Configure un proyecto de CodeBuild que lea su repositorio de CodeCommit, construya la
imagen Docker usando el `buildspec.yml` incluido en el código, y la publique en su
repositorio de ECR. Ejecute el build y verifique que la imagen aparece en ECR con la
etiqueta `latest`.

::: solucion
1. En la consola de AWS, abra [**CodeBuild**](https://console.aws.amazon.com/codesuite/codebuild/home) y pulse **Create build project**.
2. En **Project name**, escriba `taller-aws-<su-nombre>-build`.
3. En **Source provider**, seleccione **AWS CodeCommit** y luego su repositorio.
4. En **Reference type**, elija **Branch → main**.
5. En **Environment → Environment image**, seleccione **Managed image**.
6. Seleccione **Operating system: Amazon Linux**, **Runtime: Standard**, la imagen
   más reciente (por ejemplo `aws/codebuild/standard:7.0`).
7. Active la casilla **Privileged mode** — sin esta opción, Docker no puede ejecutarse
   dentro del build y el proceso falla.
8. En **Service role**, seleccione **New service role**.
9. En **Additional configuration → Environment variables**, agregue:
   - `AWS_ACCOUNT_ID` = su ID de cuenta (12 dígitos)
   - `IMAGE_REPO_NAME` = `taller-aws-<su-nombre>`
   - `IMAGE_TAG` = `latest`
10. En **Buildspec**, deje **Use a buildspec file** seleccionado.
11. En **Artifacts**, seleccione **No artifacts**.
12. Pulse **Create build project**.
13. En [IAM](https://console.aws.amazon.com/iam/home), busque el rol cuyo nombre comienza con `codebuild-taller-aws-<su-nombre>`,
    adjúntele la política `AmazonEC2ContainerRegistryPowerUser`.
14. Vuelva a CodeBuild, abra el proyecto, y pulse **Start build**.
15. En la pestaña **Build logs**, siga la ejecución hasta que el estado sea
    **Succeeded**.
16. En ECR, abra su repositorio y confirme que aparece una imagen con la etiqueta
    `latest` y la fecha de hace unos minutos.
:::
