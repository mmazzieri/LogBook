using Microsoft.Extensions.DependencyInjection;
using Volo.Abp.Modularity;

namespace LogBook.EntityFrameworkCore
{
    [DependsOn(
        typeof(LogBookEntityFrameworkCoreModule)
        )]
    public class LogBookEntityFrameworkCoreDbMigrationsModule : AbpModule
    {
        public override void ConfigureServices(ServiceConfigurationContext context)
        {
            context.Services.AddAbpDbContext<LogBookMigrationsDbContext>();
        }
    }
}
