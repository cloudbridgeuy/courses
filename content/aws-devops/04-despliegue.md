+++
title = "El despliegue: CloudFormation como caja negra"
+++

## El salto de la imagen a la aplicación

En la sección anterior se construyó y publicó la imagen Docker en ECR. Pero una imagen
en un registro no es una aplicación en línea: alguien tiene que lanzar el contenedor,
colocarlo detrás de un balanceador de carga, conectarlo a una base de datos, y
configurar la red. Hacer eso paso a paso desde la consola tomaría más de una hora y
sería difícil de reproducir exactamente.

**AWS CloudFormation** resuelve este problema con un enfoque declarativo: se describe
en un archivo YAML o JSON qué recursos se quieren: un clúster ECS, un servicio Fargate,
un ALB, una tabla DynamoDB, grupos de seguridad, roles de IAM. Luego CloudFormation los
crea todos en el orden correcto, manejando las dependencias automáticamente.

## El template de esta semana

Los templates de CloudFormation del taller están en la carpeta `infra/templates`
del repositorio. Hay más de una versión de cada uno, así que conviene prestar
atención al nombre exacto que la guía menciona en cada paso.

Por ahora, el template es una **caja negra**: se lanza con unos pocos parámetros
y se obtiene un ambiente funcional en minutos. No es necesario entender su
contenido todavía —eso se abre en la Semana 2.

::: info
Según la cuenta que se use para el curso, la red se crea desde el template o se
reutilizan una VPC y subredes existentes de la cuenta. Confirmar con el
instructor cuál de las dos versiones corresponde:

- `taller-aws-devops-semana1.yaml` — crea su propia VPC y subredes.
- `taller-aws-devops-semana1-vpc-existente.yaml` — recibe la VPC y las subredes
  como parámetros.
:::

:::app
<cb-file path="./infra/templates/taller-aws-devops-semana1.yaml" type="yaml" toggleable></cb-file>
:::

:::app
<cb-file path="./infra/templates/taller-aws-devops-semana1-vpc-existente.yaml" type="yaml" toggleable></cb-file>
:::

Lo que despliega el template, en cualquiera de sus dos versiones:

- Una **tabla de DynamoDB** para la aplicación.
- Un **clúster ECS** y un **servicio Fargate** que ejecuta la imagen Docker.
- Un **Application Load Balancer** (ALB) que recibe el tráfico HTTP y lo dirige al
  contenedor.
- Los roles de IAM, grupos de seguridad, y configuración de red necesarios para que
  todo funcione junto.

::: warning
Como se mencionó, la versión estándar del template también crea recursos de red.
Verificar qué template se está usando, para no crear recursos innecesarios.
:::

Los únicos parámetros configurables son el **nombre del stack** y el **URI de la
imagen en ECR**. La variante de VPC existente pide además la red, como se detalla
abajo. El resto lo gestiona el template.

::: extra Si la cuenta ya tiene una VPC que se debe reutilizar
El template estándar crea su propia VPC por participante. En cuentas donde eso no
es viable (la cuota por defecto es de 5 VPC por región, o la organización exige
usar una red existente) existe una variante llamada
`taller-aws-devops-semana1-vpc-existente.yaml`, que en lugar de crear la red, pide
tres parámetros más: la **VPC** y **dos subredes públicas en zonas de
disponibilidad distintas** (las de la VPC por defecto se pueden utilizar), que la consola ofrece como desplegables. Todo lo demás es idéntico, y al
borrar el stack la red de la cuenta queda intacta. Si la VPC disponible no tiene
subredes públicas, el template `taller-aws-devops-extra-subredes-publicas.yaml`
se puede utilizar para desplegar por una sola vez las subredes necesarias.
:::

:::app
<cb-file path="./infra/templates/taller-aws-devops-extra-subredes-publicas.yaml" type="yaml" toggleable></cb-file>
:::


## Un detalle que vale la pena notar

Al abrir la URL del ALB en el navegador, se ve cargarse esta misma guía: la
plataforma del taller servida desde el despliegue en ECS. La aplicación que
se construyó, desplegó, y opera es exactamente el entorno desde el que se leen estas
instrucciones. No es un ejemplo genérico, es el sistema real.

:::slide
## El template de la Semana 1

Un template, dos parámetros, un ambiente completo:

- DynamoDB · ECS + Fargate · Application Load Balancer
- Roles de IAM, grupos de seguridad, red

Los parámetros configurables son el **nombre del stack** y el **URI de la imagen**.
:::

:::slide light
## Templates de CloudFormation

**Creación de VPC:**
:::app
<cb-file path="./infra/templates/taller-aws-devops-semana1.yaml" type="yaml" toggleable></cb-file>
:::

**Consume configuración de VPC:**
:::app
<cb-file path="./infra/templates/taller-aws-devops-semana1-vpc-existente.yaml" type="yaml" toggleable></cb-file>
:::
:::

:::slide
## Limitaciones de las VPC

- La cuota por defecto es de **5 VPC por región**
- Muchas cuentas exigen reutilizar una **red existente**, como la VPC por defecto.
- Por eso hay dos versiones del template:
  - **Estándar**: crea su propia VPC y subredes.
  - **`vpc-existente`**: recibe la VPC y **dos subredes públicas** como parámetros.
- La variante no toca la red de la cuenta: al borrar el stack, queda **intacta**.
:::


## Práctica guiada: lanzar el stack de CloudFormation

### Abrir CloudFormation

1. Abrir [**CloudFormation**](https://console.aws.amazon.com/cloudformation/home).
2. Confirmar que la región seleccionada (esquina superior derecha) es la misma que se ha
   usado para todos los recursos del taller.

### Crear el stack

1. Pulsar **Create stack** y seleccionar **With new resources (standard)**.
2. En la sección **Specify template**, seleccionar **Upload a template file**.
3. Pulsar **Choose file** y seleccionar el archivo `taller-aws-devops-semana1.yaml` o
   `taller-aws-devops-semana1-vpc-existente.yaml`.
4. Pulsar **Next**.

### Completar los parámetros

1. En **Stack name**, escribir `taller-aws-<su-nombre>` (por ejemplo: `taller-aws-maria`). El
   nombre del stack identifica el ambiente en la consola y debe ser único en la región.
2. En los parámetros del template, localizar el campo correspondiente al **URI de
   la imagen**. Pegar el URI completo copiado de ECR al final de la sección anterior.
   El formato es similar a:
   ```
   123456789012.dkr.ecr.us-east-1.amazonaws.com/taller-aws-<su-nombre>:latest
   ```

   Con la `awscli`:
   ```bash
   ❯ echo "$(aws ecr describe-repositories \
      --repository-names $TALLER \
      --query "repositories[0].repositoryUri" \
      --output text):latest"
   410228653321.dkr.ecr.us-east-2.amazonaws.com/taller-aws-guzman:latest
    ```

3. Agregar los parámetros de la VPC y las subredes, en caso de ser necesario.

   Con la `awscli`:

   ```bash
   ❯ VPC=$(aws ec2 describe-vpcs \
      --filters Name=is-default,Values=true \
      --query "Vpcs[0].VpcId" \
      --output text)

   ❯ echo $VPC
   vpc-032fa70b7d0853607

   ❯ aws ec2 describe-subnets \
      --filters Name=vpc-id,Values="$VPC" \
      --query "sort_by(Subnets,&AvailabilityZone)[].[SubnetId,AvailabilityZone,MapPublicIpOnLaunch]" \
      --output table
    ----------------------------------------------------
    |                  DescribeSubnets                 |
    +---------------------------+--------------+-------+
    |  subnet-038ffa1c748d77f6a |  us-east-2a  |  True |
    |  subnet-086bf6c440e2d8f1f |  us-east-2b  |  True |
    |  subnet-05dc99ef66e6804ac |  us-east-2c  |  True |
    +---------------------------+--------------+-------+
    ```

::: info
Si el template funciona bien, la consola debería ofrecer automáticamente estos valores.
:::

4. Revisar los demás parámetros. Dejarlos con sus valores predeterminados a menos que se
   indique lo contrario.
5. Pulsar **Next**.

### Confirmar y lanzar

1. En la pantalla de revisión, desplazarse hasta la sección **Capabilities** al pie
   de la página. Se verá un aviso sobre que el template puede crear recursos de IAM.
   Marcar la casilla **I acknowledge that AWS CloudFormation might create IAM
   resources**. Luego **Next**.
2. Pulsar **Submit** en la página de resumen.

### Seguir la creación del stack

1. CloudFormation lleva automáticamente a la vista del stack recién iniciado.
   Seleccionar la pestaña **Events**. Se ve cómo se van creando los recursos en tiempo
   real, uno por uno, con su estado.
2. Esperar hasta que el estado del stack (en la parte superior) cambie a
   **CREATE_COMPLETE**. El proceso toma entre 3 y 8 minutos, dependiendo de la región.
   Si algún recurso falla, el estado cambia a **ROLLBACK_IN_PROGRESS** y CloudFormation
   deshará los cambios automáticamente —revisar el evento fallido para entender el motivo.

### Obtener la URL de la aplicación

1. Una vez en **CREATE_COMPLETE**, seleccionar la pestaña **Outputs**.
2. Se verá una salida llamada `ALBUrl` (o similar, según el template). Copiar el valor
    —es la URL pública del Application Load Balancer.
3. Abrir esa URL en una nueva pestaña del navegador. En unos segundos se ve cargarse
    esta guía del taller, servida desde el contenedor recién desplegado en ECS.

---

{#ejercicio-7}
### Ejercicio 7 — Desplegar la aplicación

Lanzar el stack de CloudFormation con el template
`taller-aws-devops-semana1.yaml`
o `taller-aws-devops-semana1-vpc-existente.yaml`. Usar como URI de la imagen el
valor copiado de ECR al terminar el Ejercicio 4, y los ID de la VPC y de las
subredes indicadas. Al terminar, abrir la URL del ALB en el navegador y confirmar que la
aplicación está en línea.

::: solucion
1. En la consola de AWS, abrir [**CloudFormation**](https://console.aws.amazon.com/cloudformation/home).
2. Pulsar **Create stack → With new resources (standard)**.
3. Seleccionar **Upload a template file**, pulsar **Choose file**, y subir
   `taller-aws-devops-semana1.yaml` o `taller-aws-devops-semana1-vpc-existente.yaml`.
4. Pulsar **Next**.
5. En **Stack name**, escribir `taller-aws-<su-nombre>`.
6. En el campo del URI de la imagen, pegar el URI completo de ECR con la etiqueta
   `latest` (por ejemplo:
   `123456789012.dkr.ecr.us-east-1.amazonaws.com/taller-aws-maria:latest`).

   Con la `awscli`:
   ```bash
   ❯ echo "$(aws ecr describe-repositories \
      --repository-names $TALLER \
      --query "repositories[0].repositoryUri" \
      --output text):latest"
   410228653321.dkr.ecr.us-east-2.amazonaws.com/taller-aws-guzman:latest
    ```

7. Agregar los parámetros de la VPC y las subredes, en caso de ser necesario.

   Con la `awscli`:

   ```bash
   ❯ VPC=$(aws ec2 describe-vpcs \
      --filters Name=is-default,Values=true \
      --query "Vpcs[0].VpcId" \
      --output text)

   ❯ echo $VPC
   vpc-032fa70b7d0853607

   ❯ aws ec2 describe-subnets \
      --filters Name=vpc-id,Values="$VPC" \
      --query "sort_by(Subnets,&AvailabilityZone)[].[SubnetId,AvailabilityZone,MapPublicIpOnLaunch]" \
      --output table
    ----------------------------------------------------
    |                  DescribeSubnets                 |
    +---------------------------+--------------+-------+
    |  subnet-038ffa1c748d77f6a |  us-east-2a  |  True |
    |  subnet-086bf6c440e2d8f1f |  us-east-2b  |  True |
    |  subnet-05dc99ef66e6804ac |  us-east-2c  |  True |
    +---------------------------+--------------+-------+
    ```

::: info
Si el template funciona bien, la consola debería ofrecer automáticamente estos valores.
:::
8. Pulsar **Next** para llegar a la pantalla de revisión.
9. En la sección **Capabilities**, marcar la casilla de aceptación de recursos de IAM.
10. Pulsar **Submit**.
11. En la pestaña **Events**, esperar a que el estado del stack llegue a
    **CREATE_COMPLETE**.
12. En la pestaña **Outputs**, copiar el valor de **ALBUrl**.
13. Abrir esa URL en el navegador. Se verá la plataforma del taller corriendo desde el
    despliegue.
:::

:::slide light
{{ejercicio-7}}
:::
