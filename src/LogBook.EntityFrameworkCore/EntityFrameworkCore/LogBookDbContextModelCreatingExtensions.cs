using Microsoft.EntityFrameworkCore;
using Volo.Abp;

namespace LogBook.EntityFrameworkCore
{
    public static class LogBookDbContextModelCreatingExtensions
    {
        public static void ConfigureLogBook(this ModelBuilder builder)
        {
            Check.NotNull(builder, nameof(builder));

            /* Configure your own tables/entities inside here */

            //builder.Entity<YourEntity>(b =>
            //{
            //    b.ToTable(LogBookConsts.DbTablePrefix + "YourEntities", LogBookConsts.DbSchema);
            //    b.ConfigureByConvention(); //auto configure for the base class props
            //    //...
            //});
        }
    }
}