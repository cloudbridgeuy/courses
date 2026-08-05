+++
title = "Cierre del curso"
+++

:::inline-slide with-title light
## Mandamientos DevOps

Existen varios manifestos o principios de DevOps que intentan recopilar reglas basicas a seguir.

Por ejemplo:

| # | Mandamiento |
| --- | --- |
| 1 | Si no está monitorizado, no está en producción. |
| 2 | El trabajo no planificado le roba tiempo al planificado. |
| 3 | En TI estamos todos juntos. |
| 4 | Los requisitos operativos importan tanto como los funcionales. |
| 5 | Sacarlo a producción. |
| 6 | Llamarlo DevOps, NoOps o Devs & Ops. Se trata de personas. |
| 7 | La guardia es de todos. |
| 8 | Filosofía o marco de trabajo: hacer lo que funciona y vuelve eficiente al negocio. |
| 9 | DevOps abarca desarrollo, operaciones, dirección, administración, ventas, limpieza, diseño gráfico, baristas y más. |
| 10 | Leer *The Phoenix Project*, *The Goal* y *Critical Chain*. |
| 11 | Conseguir que el jefe lea *The Phoenix Project*, *The Goal* y *Critical Chain*. |
| 12 | «Proceso» no es una mala palabra. |
| 13 | Integrar continuamente, porque todo se puede romper. |
| 14 | Desplegar continuamente, porque el despliegue es un proceso central y la práctica hace al maestro. |
| 15 | Comunicar continuamente, porque las personas están en el centro del proceso. |
| 16 | Identificar los cuellos de botella. |
| 17 | Automatizar todo… salvo que se opere una central nuclear. |
| 18 | Automatizar todo igual. |

[Fuente: DevOps Madrid](https://madrid.devops.es/mandamientos-devops)

No son reglas: son recordatorios, escritos para
discutirse. Casi todos describen algo que el taller abordo.
:::

:::inline-slide
## De dónde vienen estas frases

:::skip
Ninguna de las dieciocho es original. Todas comprimen ideas que se publicaron antes, con
autor y fecha. Conocer el linaje evita tratarlas como dogma:
:::


| Año | Documento | Autoría | Qué aportó |
| --- | --- | --- | --- |
| 1984 | *The Goal* | Eliyahu Goldratt | La teoría de restricciones: un sistema va tan rápido como su cuello de botella. Origen del mandamiento 16. |
| 1997 | *Critical Chain* | Eliyahu Goldratt | La misma teoría aplicada a proyectos: las estimaciones no fallan por optimismo, fallan por multitarea. |
| 2001 | **Manifiesto Ágil** | 17 firmantes, Snowbird | Cuatro valores y doce principios. El formato «lista corta de frases discutibles» que los mandamientos imitan. |
| 2010 | **CAMS** | Damon Edwards y John Willis, tras el primer DevOpsDays de EE. UU. | Cultura, Automatización, Medición, Compartir. Jez Humble le agregó la *L* de *Lean*: **CALMS**. |
| 2011 | **The Twelve-Factor App** | Adam Wiggins (Heroku) | Doce reglas sobre cómo debe construirse la aplicación para que se pueda operar. |
| 2012 | **Las tres vías** | Gene Kim | Flujo, retroalimentación, y aprendizaje continuo: los tres principios bajo todas las prácticas DevOps. |
| 2013 | *The Phoenix Project* | Gene Kim, Kevin Behr, George Spafford | *The Goal* reescrita en un departamento de TI. Origen de los mandamientos 2, 10 y 11. |
| 2016 | *The DevOps Handbook* | Kim, Debois, Willis, Humble | Las tres vías convertidas en prácticas concretas. |

:::

A diferencia del manifiesto Agile, no existe un equivalente para DevOps

Asi, que.

![Hay 14 estándares en competencia. Alguien propone uno universal que cubra todos los
casos. Ahora hay 15 estándares en competencia.](/static/standards.png)

[Fuente: xkcd 927 — *Standards*](https://xkcd.com/927/) (CC BY-NC 2.5)

Este es el mío.

:::inline-slide
## Principios de DevOps Personales

1. Es mejor monitorear 3 variables críticas de la aplicación, que 100 estandar.
2. Todos nuestros procesos tienen que tener la capacidad de ser automatizados.
3. Operaciones y desarrollo deben trabajar juntos.
4. Los clientes de Operaciones son los desarrolladores y el negocio.
5. Es mejor contar con tres tests en CI analizando camnios criticos que 1000 unit tests que miden nada.
6. Es importante demistificar la salida a producción.
7. Priorizar tareas que no atacan el cuello de botella actual es una perdida de tiempo.
8. El personal de DevOps debe saber programar.
9. Nunca dejes a un agente que corrar despliegues por vos, pero no tengas miedo de usarlos.
:::

:::inline-slide
## Las doce reglas de una aplicación

[Fuente: 12factor.net/es](https://12factor.net/es/)

Son la contraparte de estos mandamientos del lado del código, los cuales esta bueno
que se compartan entre desarrollo y operaciones.

| # | Factor | La regla |
| --- | --- | --- |
| I | Código base | Un código base bajo control de versiones, muchos despliegues. |
| II | Dependencias | Declarar y aislar las dependencias de forma explícita. |
| III | Configuración | Guardar la configuración en el entorno. |
| IV | *Backing services* | Tratar los servicios de respaldo como recursos conectables. |
| V | Construir, publicar, ejecutar | Separar de forma estricta las etapas de construcción y ejecución. |
| VI | Procesos | Ejecutar la aplicación como uno o más procesos sin estado. |
| VII | Asignación de puertos | Exponer los servicios mediante la asignación de puertos. |
| VIII | Concurrencia | Escalar horizontalmente mediante el modelo de procesos. |
| IX | Desechabilidad | Maximizar la robustez con inicios rápidos y apagados seguros. |
| X | Paridad desarrollo/producción | Mantener desarrollo, preproducción, y producción lo más parecidos posible. |
| XI | Registros | Tratar los registros como flujos de eventos. |
| XII | Procesos de administración | Ejecutar las tareas de gestión como procesos de un solo uso. |

Buena parte del taller las asume sin nombrarlas.
:::

:::inline-slide
## En el curso

| # | Factor | Dónde apareció |
| --- | --- | --- |
| I | Código base | Un repositorio en CodeCommit, muchos despliegues del mismo commit. |
| II | Dependencias | El `Dockerfile` las declara y las aísla; nada se instala en el servidor. |
| III | Configuración | Variables de entorno en la task definition; los secretos con `secrets` y `valueFrom`, nunca en el template. |
| IV | *Backing services* | La tabla de DynamoDB llega por variable de entorno: se puede cambiar sin tocar el código. |
| V | Construir, publicar, ejecutar | Las tres etapas separadas del pipeline. La imagen se construye una vez y se promueve. |
| VI | Procesos | Sin estado en la tarea: todo el estado vive en DynamoDB, por eso escalar es sumar copias. |
| VII | Asignación de puertos | El contenedor publica un puerto; el target group lo consume. |
| VIII | Concurrencia | `DesiredCount` y el auto scaling: se escala sumando procesos, no agrandándolos. |
| IX | Desechabilidad | Arranque rápido y apagado limpio: el drenaje ante `SIGTERM`, y `StopTimeout`. |
| X | Paridad desarrollo/producción | La misma imagen corre local y en Fargate. |
| XI | Registros | Logs a `stdout`; el driver `awslogs` los lleva a CloudWatch. La aplicación no escribe archivos. |
| XII | Procesos de administración | Los subcomandos del binario (`echo`, `healthcheck`) corren como tareas de un solo uso. |

Los factores III, VI, IX y XI son los que más se sienten en operación: son la diferencia
entre un servicio que se puede reiniciar, escalar y mover, y uno que hay que cuidar.
:::

:::inline-slide light
## Qué sigue, más allá del taller

Lo construido es una base sólida, no el final del camino. Hacia dónde seguir:

- *Despliegues sin interrupción*: estrategias *blue/green* con CodeDeploy,
  para desplegar sin cortar el servicio ni arriesgar un rollback manual.
- *El pipeline también como código*: definir el propio pipeline en
  CloudFormation, de modo que la automatización sea tan reproducible como la
  infraestructura que despliega.
- *Múltiples ambientes*: separar desarrollo, *staging*, y producción, cada
  uno con su stack y su rama, promoviendo cambios entre ellos.
- *Pruebas en el pipeline*: agregar una etapa de pruebas
  automáticas entre Build y Deploy, para que solo avance lo que pasa las verificaciones.
- *Objetivos de nivel de servicio*: convertir las alarmas de hoy
  en SLO con presupuesto de error, para que «está lento» tenga un umbral acordado.
- *Ensayos de falla*: repetir el ejercicio final en un día tranquilo,
  a propósito, con el equipo mirando.
:::

::: extra Más allá del YAML a mano
El taller escribió CloudFormation directo, y eso es lo correcto para aprender: es la
capa sobre la que se apoya todo lo demás. En un equipo, el YAML a mano rara vez es la
última palabra. Tres caminos, de menor a mayor distancia del archivo original:

- **`Transform`** — una sección que le pide a CloudFormation procesar el template antes
  de desplegarlo. `AWS::Serverless-2016-10-31` activa **SAM**, que convierte veinte
  líneas de función Lambda, rol, y API en cinco. `AWS::LanguageExtensions` agrega lo que
  falta en el lenguaje base, como bucles (`Fn::ForEach`) e `Fn::Length`. El resultado
  sigue siendo un stack de CloudFormation, con sus change sets y sus eventos.
- **CDK** — define la infraestructura en TypeScript, Python, o Java, y **sintetiza** un
  template de CloudFormation. Se gana lo que da un lenguaje de verdad —clases, tests,
  el autocompletado del editor— y se paga con una capa más entre lo que se escribe y lo
  que se despliega. Todo lo de esta semana sigue aplicando: `cdk deploy` crea un stack
  de CloudFormation normal, y cuando algo falla se lee la pestaña **Events**.
- **Terraform** — de HashiCorp, ajeno a AWS y por eso capaz de gestionar varios
  proveedores con un solo lenguaje. Guarda el estado en un archivo propio en vez de
  dejarlo en el servicio, lo que da más control y agrega una responsabilidad: ese
  archivo hay que guardarlo, bloquearlo, y no perderlo. Su `terraform plan` es el
  equivalente exacto del change set.

Los conceptos no cambian entre las tres: estado deseado contra estado real,
reconciliación, vista previa antes de aplicar, dependencias entre recursos, y ciclos de
vida distintos en stacks distintos. Aprendida la idea en CloudFormation, cambiar de
herramienta es cambiar de sintaxis.
:::

---

## Desarmar el ambiente

Queda un último paso, y no es un trámite administrativo: el teardown es donde
«automatizar todo» (17 y 18) muestra la factura de lo que no se automatizó. Al terminar
el taller, el ambiente son **cinco stacks** y una tabla que sobrevive a todos ellos.
Borrarlos en cualquier orden no funciona.

La razón se explicó en la Semana 2: mientras un stack importe un export, ese export no
se puede borrar. Empezar por el stack de red termina en un `DELETE_FAILED` con un
mensaje explícito:

```
Export taller-aws-maria-red-vpc-id cannot be deleted as it is in use by
taller-aws-maria-plataforma
```

No es una falla: es la garantía funcionando. El orden de borrado es el orden de
creación, al revés.

Un export no se borra mientras alguien lo importe.

**Borrar es crear, al revés.**

### Antes de empezar: apagar lo que reconstruye

El pipeline vigila `main` y despliega sobre el servicio de ECS. Si se dispara en medio
del teardown, vuelve a tocar recursos que se están borrando.

1. Abrir [**CodePipeline**](https://console.aws.amazon.com/codesuite/codepipeline/home),
   entrar a `taller-aws-<su-nombre>-pipeline`, y pulsar **Delete pipeline**. La regla de
   notificación de la Semana 3 se va con él.
2. El auto scaling no hace falta tocarlo: la política vive dentro del servicio, y
   desaparece con el stack de aplicación.

### Borrar los stacks, de arriba hacia abajo

En [**CloudFormation**](https://console.aws.amazon.com/cloudformation/home), borrar en
este orden, esperando el `DELETE_COMPLETE` de cada uno antes de seguir:

| Orden | Stack | Qué se lleva |
| --- | --- | --- |
| 1 | `taller-aws-<su-nombre>-eco` (si se creó) | El servicio de eco, su task definition, su target group, su regla del listener, y su grupo de logs. |
| 2 | `taller-aws-<su-nombre>-app` | Lo mismo, para la aplicación principal. |
| 3 | `taller-aws-<su-nombre>-datos` | El stack, pero **no la tabla**. |
| 4 | `taller-aws-<su-nombre>-plataforma` | El clúster, el balanceador, los listeners, y —si se activó HTTPS— el certificado de ACM y los registros de Route 53. |
| 5 | `taller-aws-<su-nombre>-red` | La VPC, las subredes, y los grupos de seguridad. |

Los stacks 1 y 2 se pueden borrar a la vez —ninguno importa nada del otro—, y lo mismo
vale para el 3 y el 4 entre sí. Lo que no se puede es adelantar el 5.

Con la CLI, el orden se expresa esperando:

```bash
TALLER=taller-aws-<su-nombre>
for stack in "$TALLER-eco" "$TALLER-app" "$TALLER-datos" \
             "$TALLER-plataforma" "$TALLER-red"; do
  aws cloudformation delete-stack --stack-name "$stack"
  aws cloudformation wait stack-delete-complete --stack-name "$stack"
done
```

::: warning
Si el stack de red queda en `DELETE_FAILED` sobre una subred o un grupo de seguridad,
casi siempre es una interfaz de red (ENI) que todavía no se liberó —una tarea de Fargate
que tarda en apagarse—. Esperar un par de minutos y reintentar el borrado suele alcanzar.
:::

### La tabla que sobrevive

Al borrar el stack de datos, la pestaña **Events** muestra la tabla como
`DELETE_SKIPPED`. Es exactamente lo que se pidió en la migración con
`DeletionPolicy: Retain`, y recién ahora se ve el otro lado de esa decisión: la tabla
queda **sin stack y sin dueño**, viva, facturando, y ocupando su nombre.

Ese es el precio de `Retain`, y se paga a mano:

```bash
aws dynamodb delete-table --table-name "$TABLA"
```

La lección operativa: `Retain` no es "más seguro" sin más. Traslada la responsabilidad
del borrado de CloudFormation a una persona. Un recurso retenido y olvidado es una
factura que nadie revisa — trabajo no planificado (mandamiento 2), agendado para dentro
de tres meses.

### Lo que nunca estuvo en un stack

Los cinco stacks no cubren todo el taller. Buena parte de la Semana 1, y de la Semana 4,
se creó a mano por la consola, y por eso no se borra sola:

| Recurso | Servicio |
| --- | --- |
| Repositorio `taller-aws-<su-nombre>` | CodeCommit |
| Proyecto `taller-aws-<su-nombre>-build` | CodeBuild |
| Repositorio `taller-aws-<su-nombre>`, con sus imágenes | ECR |
| El bucket de artefactos que creó CodePipeline (`codepipeline-<región>-…`) | S3 |
| El dashboard, y la alarma `cpu-alta-<su-nombre>` | CloudWatch |
| El grupo de logs `/aws/codebuild/taller-aws-<su-nombre>-build` | CloudWatch Logs |
| Los roles de servicio que crearon las consolas (`codebuild-…`, `AWSCodePipelineServiceRole-…`) | IAM |
| El tipo `CloudBridge::Taller::App::MODULE`, si se hizo la práctica de módulos | CloudFormation Registry |

Un bucket de S3 con objetos no se borra hasta vaciarlo, y un repositorio de ECR con
imágenes pide `--force`.

El módulo se saca del registro al revés de como entró: primero cada versión que no
sea la default, y al final el tipo entero, que se lleva la default consigo:

```bash
aws cloudformation deregister-type --type MODULE \
  --type-name CloudBridge::Taller::App::MODULE --version-id "00000001"
aws cloudformation deregister-type --type MODULE \
  --type-name CloudBridge::Taller::App::MODULE
```

Y `cfn submit` dejó su propio stack, `CloudFormationManagedUploadInfrastructure`,
con dos buckets adentro. Como todo bucket, hay que vaciarlos antes de que el stack
se deje borrar.

::: extra La lista se hace sola cuando todo es un stack
Esta tabla existe porque el taller creó recursos por la consola para poder enseñarlos
uno a uno. Un ambiente definido enteramente como código no la necesita: se borran los
stacks en orden inverso, y no queda nada.

Ese es el argumento más práctico a favor de la infraestructura como código, y es el
único que solo se aprecia al desarmar: **lo que no está en un template se borra a mano,
o no se borra.**

Los mandamientos 17 y 18 se leen como una exageración graciosa hasta que aparece esta
tabla. Entonces se leen como una advertencia.
:::

### Confirmar que no queda nada

1. En [**CloudFormation**](https://console.aws.amazon.com/cloudformation/home), filtrar
   por `taller-aws-<su-nombre>`. La lista debe quedar vacía.
2. Abrir la última **UrlBase** conocida: el navegador debe dar un error de conexión.
3. Pasar un barrido por lo que suele quedar atrás:

   ```bash
   aws dynamodb list-tables \
     --query "TableNames[?contains(@, '$TALLER')]"
   aws ecr describe-repositories \
     --query "repositories[?contains(repositoryName, '$TALLER')].repositoryName"
   aws logs describe-log-groups --log-group-name-prefix "/ecs/$TALLER" \
     --query "logGroups[].logGroupName"
   ```

   Tres listas vacías cierran el taller.

## Del código a la operación

Se construyó, desplegó, automatizó, y observó un sistema real en AWS. Las dieciocho
frases del principio no son la conclusión del taller: son el índice de las conversaciones
que quedan por tener, ahora que hay un sistema concreto sobre el cual tenerlas.

:::slide
## Del código a la operación

**-Muchas Gracias**
:::
