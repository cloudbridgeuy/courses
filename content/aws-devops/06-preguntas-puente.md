+++
title = "Preguntas puente"
+++

Estas preguntas abren la sesión del viernes. Conviene pensarlas después de la sesión del
miércoles, cuando el ambiente todavía está fresco. No se buscan las respuestas de
inmediato: se razonan desde lo que se construyó. Al comenzar la sesión remota,
cada participante comparte su respuesta y se discute en conjunto antes de continuar.

:::slide
## Preguntas puente

1. ¿Qué hace CodeBuild con el `buildspec.yml`, fase por fase, y dónde queda el
   resultado?
2. Si se borra el stack, ¿qué sobrevive: CodeCommit, ECR, ambos, ninguno?
3. El template desplegó la aplicación tras un ALB: ¿qué pasos manuales se ahorró?
:::

---

## Pregunta 1

¿Qué hace exactamente CodeBuild con el `buildspec.yml`, fase por fase, y dónde queda
el resultado?

::: solucion
CodeBuild lee el archivo `buildspec.yml` desde la raíz del repositorio de CodeCommit
y ejecuta los comandos de cada fase en secuencia, dentro de un contenedor efímero
(un entorno limpio que se destruye al terminar el build):

- **`install`**: prepara el entorno de ejecución. En el `buildspec.yml` del taller,
  esta fase verifica que Docker y la CLI de AWS estén disponibles — la imagen
  administrada de CodeBuild ya los trae, así que no hay nada que instalar.
- **`pre_build`**: se ejecuta antes de la construcción principal. En el `buildspec.yml`
  del taller, esta fase autentica con Amazon ECR usando las credenciales del rol de
  IAM del proyecto. Sin este paso, el `docker push` posterior fallaría por falta de
  permisos.
- **`build`**: ejecuta `docker build` usando el `Dockerfile` en la raíz del
  repositorio, produciendo una imagen local dentro del entorno de CodeBuild. Luego
  etiqueta esa imagen con el URI completo del repositorio de ECR.
- **`post_build`**: ejecuta `docker push` para transferir la imagen etiquetada desde
  el entorno efímero de CodeBuild hacia el repositorio privado de ECR.

El resultado —la imagen Docker— queda almacenado en **Amazon ECR**, identificado por
el URI del repositorio y la etiqueta definida en la variable `IMAGE_TAG` (en este
caso, `latest`). El entorno de CodeBuild en sí desaparece al terminar: no hay
servidores que persistan entre builds.
:::

---

## Pregunta 2

Si se borra el stack de CloudFormation, ¿qué sobrevive —el repositorio de CodeCommit,
la imagen en ECR, ambos, o ninguno? ¿Por qué?

::: solucion
**Sobreviven ambos**: el repositorio de CodeCommit y la imagen en ECR.

La razón es que CloudFormation solo gestiona los recursos que se declararon en el template.
El template `taller-semana1.yaml` describe el clúster ECS, el servicio Fargate, el
Application Load Balancer, la tabla de DynamoDB, los roles de IAM y la configuración
de red. Esos recursos los creó CloudFormation y los elimina cuando se borra el stack.

El repositorio de CodeCommit y el repositorio de ECR (junto con la imagen que contiene)
se crearon directamente desde la consola, fuera de cualquier stack de CloudFormation.
CloudFormation no los conoce y no los toca. Por eso al recrear el stack basta con
proporcionar de nuevo el URI de la imagen: la imagen ya está en ECR, exactamente como
la dejó el build.

Esta separación es intencional: el código fuente y los artefactos de build tienen un
ciclo de vida distinto al del ambiente de ejecución. Los primeros crecen con cada
commit y cada build; el segundo puede destruirse y recrearse cuantas veces sea necesario.
:::

---

## Pregunta 3

El template desplegó la aplicación detrás de un ALB. ¿Qué pasos manuales se ahorró,
y cuáles de esos recursos se reconocen en la consola?

::: solucion
El template automatizó al menos los siguientes pasos que, de otro modo, habría que
ejecutar manualmente desde la consola:

1. Crear la tabla de DynamoDB con el nombre y la configuración de clave correctos.
2. Crear el clúster de ECS.
3. Definir la **task definition** de Fargate: especificar la imagen, los límites de
   CPU y memoria, las variables de entorno, el rol de ejecución.
4. Crear el **servicio ECS** que mantiene el número de tareas en ejecución y las
   reemplaza si fallan.
5. Crear el **Application Load Balancer**, el listener en el puerto 80, y el target
   group que apunta al servicio ECS.
6. Crear los **grupos de seguridad** que permiten el tráfico HTTP hacia el ALB y del
   ALB hacia los contenedores.
7. Crear el **rol de IAM de ejecución de ECS** para que Fargate pueda descargar la
   imagen de ECR.
8. Conectar todo: el ALB al target group, el target group al servicio, el servicio a
   la task definition, la task definition a la imagen en ECR.

En la consola se puede verificar cada uno de estos recursos directamente:

- **ECS → Clusters**: se verá el clúster y, dentro de él, el servicio y las tareas en
  estado `RUNNING`.
- **EC2 → Load Balancers**: se verá el ALB con su DNS público.
- **DynamoDB → Tables**: se verá la tabla creada por el template.
- **IAM → Roles**: se verá el rol de ejecución de ECS cuyo nombre contiene el nombre del
  stack.

El valor de CloudFormation es que todos esos pasos, incluyendo el orden correcto de
creación y las dependencias entre recursos, quedan codificados en el archivo YAML.
Reproducirlos requiere lanzar el template, no recordar los pasos.
:::

---

## Dónde estamos

Al cerrar la Semana 1, cada participante tiene el flujo completo de la primera parte
del taller funcionando de punta a punta:

- Un **repositorio en CodeCommit** con el código de la aplicación, versionado con git.
- Un **pipeline de build en CodeBuild** que construye la imagen Docker y la publica
  en **ECR** a partir del `buildspec.yml`.
- La **aplicación en línea** sobre ECS/Fargate detrás de un ALB, desplegada con un
  template de CloudFormation.
- El **ciclo de recuperación** practicado: destruir y recrear el ambiente en minutos.

Se construyó, desplegó, y operó el sistema. Lo que todavía es una caja negra es **cómo**
ese template arma todo por dentro.

## Qué sigue en la Semana 2

La próxima semana se abre la caja negra. Se va a:

- Leer el template `taller-semana1.yaml` recurso por recurso, y entender la
  **infraestructura como código**: parámetros, recursos, salidas, y funciones
  intrínsecas.
- **Actualizar** el stack de forma segura con *change sets*, y ver cómo CloudFormation
  maneja cambios, *drift*, y *rollback*.
- Conocer los **primeros contenedores** por dentro: las *task definitions* y los
  *services* de ECS/Fargate que el template creó.

Al final de la Semana 2 se comprenderá, y se podrá modificar, el ambiente que
esta semana solo se lanzó.
